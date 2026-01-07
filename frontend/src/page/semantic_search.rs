use gloo_net::http::Request;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen_futures::spawn_local;

const API_BASE: &str = "http://127.0.0.1:9988";

#[derive(Debug, Clone, Serialize)]
struct SearchPayload {
    query: String,
}

#[derive(Debug, Clone, Deserialize)]
struct SearchItem {
    distance: f64,
    id: i64,
    product_id: String,
    review_body: String,
    review_rating: i64,
    review_title: String,
}

#[derive(Debug, Clone, Deserialize)]
struct SearchResponse {
    data: Vec<SearchItem>,
    status: bool,
}

#[component]
pub fn SemanticSearchPage() -> impl IntoView {
    let (query, set_query) = signal(String::new());

    let (loading, set_loading) = signal(false);
    let (error_msg, set_error_msg) = signal::<Option<String>>(None);
    let (items, set_items) = signal::<Vec<SearchItem>>(vec![]);

    // ✅ ฟังก์ชันกลาง: ไม่รับ argument เพื่อเรียกได้จากหลาย event
    let run_search = move || {
        let q = query.get().trim().to_string();
        if q.is_empty() {
            set_error_msg.set(Some("Please enter query.".to_string()));
            set_items.set(vec![]);
            return;
        }

        set_loading.set(true);
        set_error_msg.set(None);

        spawn_local(async move {
            let url = format!("{}/get-data", API_BASE);
            let payload = SearchPayload { query: q };

            // 1) build request
            let req = match Request::post(&url)
                .header("Content-Type", "application/json")
                .json(&payload)
            {
                Ok(r) => r,
                Err(e) => {
                    set_loading.set(false);
                    set_error_msg.set(Some(format!("Build request error: {e}")));
                    set_items.set(vec![]);
                    return;
                }
            };

            // 2) send
            let resp = match req.send().await {
                Ok(r) => r,
                Err(e) => {
                    set_loading.set(false);
                    set_error_msg.set(Some(format!("Network error: {e}")));
                    set_items.set(vec![]);
                    return;
                }
            };

            // 3) http status
            let status_code = resp.status();
            if !resp.ok() {
                set_loading.set(false);
                set_error_msg.set(Some(format!("HTTP {status_code}")));
                set_items.set(vec![]);
                return;
            }

            // 4) parse json
            match resp.json::<SearchResponse>().await {
                Ok(parsed) => {
                    set_loading.set(false);

                    if !parsed.status {
                        set_error_msg.set(Some("API status=false".to_string()));
                        set_items.set(vec![]);
                        return;
                    }
                    set_items.set(parsed.data);
                }
                Err(e) => {
                    set_loading.set(false);
                    set_error_msg.set(Some(format!("Parse JSON error: {e}")));
                    set_items.set(vec![]);
                }
            }
        });
    };

    // ✅ event handlers ที่ชนิดตรง
    let on_click_search = {
        let run_search = run_search.clone();
        move |_ev: leptos::ev::MouseEvent| run_search()
    };

    let on_keydown = {
        let run_search = run_search.clone();
        move |ev: leptos::ev::KeyboardEvent| {
            if ev.key() == "Enter" {
                ev.prevent_default();
                run_search();
            }
        }
    };

    view! {
        <>
            <div class="card">
                <h2 style="margin:0 0 10px 0;">"Vector Search"</h2>
                <p class="muted" style="margin:0 0 12px 0;">
                    "POST " <code>{format!("{}/get-data", API_BASE)}</code>
                </p>

                <div class="row" style="align-items:center;">
                    <input
                        class="input"
                        type="text"
                        placeholder="Type query เช่น instructions"
                        prop:value=move || query.get()
                        on:input=move |ev| set_query.set(event_target_value(&ev))
                        on:keydown=on_keydown
                    />
                    <button
                        class="btn"
                        type="button"
                        on:click=on_click_search
                        disabled=move || loading.get()
                    >
                        {move || if loading.get() { "Searching..." } else { "Search" }}
                    </button>
                </div>

                <Show when=move || error_msg.get().is_some() fallback=|| ()>
                    <div class="error" style="margin-top:12px;">
                        {move || error_msg.get().unwrap_or_default()}
                    </div>
                </Show>
            </div>

            <div class="card" style="margin-top:14px;">
                <h3 style="margin:0 0 10px 0;">"Results"</h3>

                <Show
                    when=move || !items.get().is_empty()
                    fallback=move || view! { <p class="muted" style="margin:0;">"No data."</p> }
                >
                    <style>
                        "
                        table { border-collapse: collapse; width: 100%; }
                        th, td { border: 1px solid #e2e8f0; text-align: left; padding: 10px; vertical-align: top; }
                        th { background: #f8fafc; }
                        td.small { width: 90px; white-space: nowrap; }
                        td.body { max-width: 520px; }
                        "
                    </style>

                    <table>
                        <thead>
                            <tr>
                                <th>"review_title"</th>
                                <th>"review_body"</th>
                                <th>"product_id"</th>
                                <th class="small">"rating"</th>
                                <th class="small">"distance"</th>
                            </tr>
                        </thead>

                        <tbody>
                            <For
                                each=move || items.get()
                                key=|it| it.id
                                children=move |it| {
                                    view! {
                                        <tr>
                                            <td>{it.review_title}</td>
                                            <td class="body">{it.review_body}</td>
                                            <td>{it.product_id}</td>
                                            <td class="small">{it.review_rating}</td>
                                            <td class="small">{format!("{:.6}", it.distance)}</td>
                                        </tr>
                                    }
                                }
                            />
                        </tbody>
                    </table>
                </Show>
            </div>
        </>
    }
}
