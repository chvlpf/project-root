use gloo_net::http::Request;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen_futures::spawn_local;

const API_BASE: &str = "http://127.0.0.1:9988";

#[derive(Debug, Clone, Serialize)]
struct CreatePayload {
    review_title: String,
    review_body: String,
    product_id: String,
    review_rating: i64,
}

#[derive(Debug, Clone, Deserialize)]
struct CreateInner {
    id: i64,
    message: String,
}

#[derive(Debug, Clone, Deserialize)]
struct CreateResponse {
    data: CreateInner,
    status: bool,
}

#[component]
pub fn SemanticCreatePage() -> impl IntoView {
    let (review_title, set_review_title) = signal(String::new());
    let (review_body, set_review_body) = signal(String::new());
    let (product_id, set_product_id) = signal(String::new());
    let (review_rating, set_review_rating) = signal::<i64>(5);

    let (loading, set_loading) = signal(false);
    let (error_msg, set_error_msg) = signal::<Option<String>>(None);
    let (ok_msg, set_ok_msg) = signal::<Option<String>>(None);

    let on_create = move |_| {
        let title = review_title.get().trim().to_string();
        let body = review_body.get().trim().to_string();
        let pid = product_id.get().trim().to_string();
        let rating = review_rating.get();

        set_error_msg.set(None);
        set_ok_msg.set(None);

        if title.is_empty() || body.is_empty() || pid.is_empty() {
            set_error_msg.set(Some("Please fill: title / body / product_id".to_string()));
            return;
        }
        if !(1..=5).contains(&rating) {
            set_error_msg.set(Some("review_rating must be 1..5".to_string()));
            return;
        }

        set_loading.set(true);

        spawn_local(async move {
            let url = format!("{}/create-data", API_BASE);
            let payload = CreatePayload {
                review_title: title,
                review_body: body,
                product_id: pid,
                review_rating: rating,
            };

            // ✅ 1) build request
            let req = match Request::post(&url)
                .header("Content-Type", "application/json")
                .json(&payload)
            {
                Ok(r) => r,
                Err(e) => {
                    set_loading.set(false);
                    set_error_msg.set(Some(format!("Build request error: {e}")));
                    return;
                }
            };

            // ✅ 2) send
            let resp = match req.send().await {
                Ok(r) => r,
                Err(e) => {
                    set_loading.set(false);
                    set_error_msg.set(Some(format!("Network error: {e}")));
                    return;
                }
            };

            // ✅ 3) http status
            let status_code = resp.status();
            if !resp.ok() {
                set_loading.set(false);
                set_error_msg.set(Some(format!("HTTP {status_code}")));
                return;
            }

            // ✅ 4) parse json
            match resp.json::<CreateResponse>().await {
                Ok(parsed) => {
                    set_loading.set(false);

                    if !parsed.status {
                        set_error_msg.set(Some("API status=false".to_string()));
                        return;
                    }

                    set_ok_msg.set(Some(format!(
                        "✅ {} (id={})",
                        parsed.data.message, parsed.data.id
                    )));
                }
                Err(e) => {
                    set_loading.set(false);
                    set_error_msg.set(Some(format!("Parse JSON error: {e}")));
                }
            }
        });
    };

    view! {
        <>
            <div class="card">
                <h2 style="margin:0 0 10px 0;">"Create Index"</h2>
                <p class="muted" style="margin:0 0 12px 0;">
                    "POST " <code>{format!("{}/create-data", API_BASE)}</code>
                </p>

                <div class="row">
                    <input
                        class="input"
                        type="text"
                        placeholder="product_id เช่น Likethis"
                        prop:value=move || product_id.get()
                        on:input=move |ev| set_product_id.set(event_target_value(&ev))
                    />
                    <input
                        class="input"
                        type="number"
                        min="1"
                        max="5"
                        placeholder="review_rating (1-5)"
                        prop:value=move || review_rating.get().to_string()
                        on:input=move |ev| {
                            let v = event_target_value(&ev).parse::<i64>().unwrap_or(5);
                            set_review_rating.set(v);
                        }
                    />
                </div>

                <div class="row" style="margin-top:10px;">
                    <input
                        class="input"
                        style="flex:1; min-width: 320px;"
                        type="text"
                        placeholder="review_title"
                        prop:value=move || review_title.get()
                        on:input=move |ev| set_review_title.set(event_target_value(&ev))
                    />
                </div>

                <div class="row" style="margin-top:10px; width:100%;">
                    <textarea
                        class="input"
                        style="flex:1; width:100%; min-height: 140px; resize: vertical;"
                        placeholder="review_body"
                        prop:value=move || review_body.get()
                        on:input=move |ev| set_review_body.set(event_target_value(&ev))
                    ></textarea>
                </div>

                <div class="row" style="margin-top:12px; align-items:center;">
                    <button class="btn" type="button" on:click=on_create disabled=move || loading.get()>
                        {move || if loading.get() { "Creating..." } else { "Create" }}
                    </button>
                    <span class="muted">"rating: " {move || review_rating.get()}</span>
                </div>

                <Show when=move || error_msg.get().is_some() fallback=|| ()>
                    <div class="error" style="margin-top:12px;">
                        {move || error_msg.get().unwrap_or_default()}
                    </div>
                </Show>

                <Show when=move || ok_msg.get().is_some() fallback=|| ()>
                    <div class="ok" style="margin-top:12px;">
                        {move || ok_msg.get().unwrap_or_default()}
                    </div>
                </Show>
            </div>
        </>
    }
}
