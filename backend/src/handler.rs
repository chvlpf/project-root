use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, Json};
use indexmap::IndexMap;
use serde_json::{to_value, Value};

use crate::model::SearchRequest;
use crate::presenter::{res_error, res_error_msg, res_success};
use crate::utils::load_model_fields;
use crate::AppState;

// fn build_embedding_text(payload: &Value) -> String {
//     let product_id = payload
//         .get("product_id")
//         .and_then(|v| v.as_str())
//         .unwrap_or("");
//     let title = payload
//         .get("review_title")
//         .and_then(|v| v.as_str())
//         .unwrap_or("");
//     let body = payload
//         .get("review_body")
//         .and_then(|v| v.as_str())
//         .unwrap_or("");
//     let rating = payload
//         .get("review_rating")
//         .map(|v| v.to_string())
//         .unwrap_or_else(|| "".into());
//
//     format!(
//         "product_id: {}\nreview_title: {}\nreview_body: {}\nreview_rating: {}",
//         product_id, title, body, rating
//     )
// }

fn build_embedding_text(payload: &Value) -> String {
    payload
        .get("review_body")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

fn parse_u64(v: &Value) -> Option<u64> {
    match v {
        Value::Number(n) => n.as_u64(),
        Value::String(s) => s.parse::<u64>().ok(),
        _ => None,
    }
}

pub async fn get_data(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SearchRequest>,
) -> impl IntoResponse {
    let json_path = "src/data/reviews.jsonl";
    let index_path = "src/data/reviews.index";

    if !Path::new(json_path).exists() {
        return res_success(Vec::<Value>::new());
    }

    // โหลด JSONL เป็น Vec + map id -> item
    let content = match fs::read_to_string(json_path) {
        Ok(c) => c,
        Err(_) => return res_success(Vec::<Value>::new()),
    };

    let mut items: Vec<Value> = Vec::new();
    let mut by_id: HashMap<u64, Value> = HashMap::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<Value>(line) {
            if let Some(id) = v.get("id").and_then(parse_u64) {
                by_id.insert(id, v.clone());
            }
            items.push(v);
        }
    }

    let model_fields = load_model_fields();
    let query = payload.query.trim();
    let top_k: usize = 10;
    // let top_k = payload.top_k.unwrap_or(10) as usize;

    // ถ้า query ว่าง -> คืน metadata อย่างเดียว (ยังเคารพ top_k)
    if query.is_empty() {
        let mapped: Vec<Value> = items
            .into_iter()
            .take(top_k)
            .map(|item| {
                let mut ordered = IndexMap::new();

                if let Some(idv) = item.get("id").cloned() {
                    ordered.insert("id".to_string(), idv);
                }

                for field in &model_fields {
                    if field == "embedding" {
                        ordered.insert(field.clone(), Value::Null);
                        continue;
                    }
                    let value = item.get(field).cloned().unwrap_or(Value::Null);
                    ordered.insert(field.clone(), value);
                }

                to_value(ordered).unwrap()
            })
            .collect();

        return res_success(mapped);
    }

    if !Path::new(index_path).exists() {
        return res_success(Vec::<Value>::new());
    }

    //  embed query
    let qvecs = {
        let mut embedder = state.embedder.lock().await;
        match embedder.embed(vec![query.to_string()], None) {
            Ok(v) => v,
            Err(e) => return res_error_msg(format!("embedding error: {}", e)),
        }
    };
    let qvec = match qvecs.get(0) {
        Some(v) => v.as_slice(),
        None => return res_error_msg("embedding error: empty query vector"),
    };

    //  search จาก FlatIndex
    let hits = {
        let index = state.index.lock().await;
        if index.dim() != qvec.len() {
            return res_error_msg("index dim mismatch with query embedding dim");
        }
        match index.search(qvec, top_k) {
            Ok(v) => v,
            Err(e) => return res_error_msg(format!("index search error: {}", e)),
        }
    };

    //  map id -> metadata + attach distance
    let results: Vec<Value> = hits
        .into_iter()
        .filter_map(|(id, distance)| {
            let item = by_id.get(&id)?.clone();

            let mut ordered = IndexMap::new();
            ordered.insert("id".to_string(), Value::Number(id.into()));

            for field in &model_fields {
                if field == "embedding" {
                    ordered.insert(field.clone(), Value::Null);
                    continue;
                }
                let value = item.get(field).cloned().unwrap_or(Value::Null);
                ordered.insert(field.clone(), value);
            }

            ordered.insert(
                "distance".to_string(),
                serde_json::Number::from_f64(distance as f64)
                    .map(Value::Number)
                    .unwrap_or(Value::Null),
            );

            Some(to_value(ordered).unwrap())
        })
        .collect();

    res_success(results)
}

pub async fn create_data(
    State(state): State<Arc<AppState>>,
    json: Result<Json<Value>, axum::extract::rejection::JsonRejection>,
) -> impl IntoResponse {
    use std::fs::OpenOptions;
    use std::io::Write;

    let model_fields = load_model_fields();

    let payload = match json {
        Ok(Json(value)) => value,
        Err(err) => return res_error(err),
    };

    //  payload ต้องเป็น object
    let obj = match payload.as_object() {
        Some(o) => o,
        None => return res_error_msg("payload must be a JSON object"),
    };

    let server_fields: HashSet<String> = ["embedding".to_string(), "id".to_string()]
        .into_iter()
        .collect();

    for f in server_fields.iter() {
        if obj.contains_key(f) {
            return res_error_msg(format!("do not provide '{}' (server will generate it)", f));
        }
    }

    let allowed_fields: HashSet<String> = model_fields
        .iter()
        .cloned()
        .filter(|f| !server_fields.contains(f))
        .collect();

    let payload_fields: HashSet<String> = obj.keys().cloned().collect();

    // required เฉพาะ field ที่ client ต้องส่ง
    for field in allowed_fields.iter() {
        if payload.get(field).is_none() {
            return res_error_msg(format!("{} is required", field));
        }
    }

    if !payload_fields.is_subset(&allowed_fields) {
        return res_error_msg("payload contains unexpected fields");
    }

    //  สร้าง embedding จากฟิลด์หลัก
    let text = build_embedding_text(&payload);
    let emb = {
        let mut embedder = state.embedder.lock().await;
        match embedder.embed(vec![text], None) {
            Ok(v) => v,
            Err(e) => return res_error_msg(format!("embedding error: {}", e)),
        }
    };
    let embedding_vec = emb.get(0).cloned().unwrap_or_default();

    //  append vector ลง FlatIndex -> ได้ id
    let id = {
        let index = state.index.lock().await;
        if index.dim() != embedding_vec.len() {
            return res_error_msg("index dim mismatch with embedding dim");
        }
        match index.append(&embedding_vec) {
            Ok(id) => id,
            Err(e) => return res_error_msg(format!("index append error: {}", e)),
        }
    };

    //  สร้าง payload สำหรับ JSONL: ใส่ฟิลด์ปกติ + id (ไม่เก็บ embedding แล้ว)
    let mut ordered = IndexMap::new();
    ordered.insert("id".to_string(), Value::Number(id.into()));

    for f in &model_fields {
        if f == "embedding" {
            continue;
        }
        if f == "id" {
            continue;
        }
        ordered.insert(f.clone(), payload.get(f).cloned().unwrap_or(Value::Null));
    }

    let json_value = match serde_json::to_value(ordered) {
        Ok(v) => v,
        Err(e) => return res_error_msg(format!("serialize payload error: {}", e)),
    };

    // --- write JSONL (append one line per object) ---
    let json_path = "src/data/reviews.jsonl";

    if let Err(err) = std::fs::create_dir_all("src/data") {
        return res_error(err);
    }

    let line = match serde_json::to_string(&json_value) {
        Ok(s) => s,
        Err(e) => return res_error_msg(format!("serialize error: {}", e)),
    };

    let mut file = match OpenOptions::new().create(true).append(true).open(json_path) {
        Ok(f) => f,
        Err(e) => return res_error_msg(format!("open file error: {}", e)),
    };

    if let Err(e) = writeln!(file, "{}", line) {
        return res_error_msg(format!("write file error: {}", e));
    }

    res_success(serde_json::json!({ "message": "create successful", "id": id }))
}
