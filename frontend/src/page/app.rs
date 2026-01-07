use leptos::prelude::*;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::{hooks::{use_location, use_navigate}, path, NavigateOptions};

use crate::page::semantic_create::SemanticCreatePage;
use crate::page::semantic_search::SemanticSearchPage;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <AppShell />
        </Router>
    }
}

#[component]
fn AppShell() -> impl IntoView {
    let loc = use_location();
    let nav = use_navigate();

    let is_search = move || loc.pathname.get() == "/";
    let is_create = move || loc.pathname.get() == "/create";

    let go = move |to: &'static str| {
        let nav = nav.clone();
        move |_| nav(to, NavigateOptions::default())
    };

    view! {
        <>
            <style>
                "
                :root{
                  --bg:#ffffff;
                  --text:#0f172a;
                  --muted:#64748b;
                  --border:#e2e8f0;
                  --soft:#f1f5f9;
                  --active:#111827;
                }
                .appbar{
                  position: sticky; top: 0; z-index: 50;
                  background: rgba(255,255,255,0.86);
                  backdrop-filter: blur(12px);
                  border-bottom: 1px solid var(--border);
                }
                .appbar-inner{
                  max-width: 1100px; margin: 0 auto; padding: 12px 16px;
                  display:flex; align-items:center; justify-content:space-between; gap: 12px;
                }
                .brand{
                  display:flex; align-items:center; gap: 10px;
                  color: var(--text); font-weight: 700; letter-spacing: .2px;
                  cursor: pointer;
                }
                .logo{
                  width: 34px; height: 34px; border-radius: 12px;
                  border: 1px solid var(--border);
                  background: linear-gradient(135deg, #e0f2fe, #ecfccb);
                  display:flex; align-items:center; justify-content:center; font-size: 16px;
                }
                .nav{
                  display:flex; align-items:center; gap: 8px; padding: 6px;
                  border: 1px solid var(--border); background: var(--soft);
                  border-radius: 14px;
                }
                .nav-btn{
                  border: 0; background: transparent; color: var(--muted);
                  font-weight: 600; padding: 8px 12px; border-radius: 12px;
                  cursor: pointer;
                  transition: background .15s ease, color .15s ease, transform .05s ease;
                }
                .nav-btn:hover{ background: rgba(255,255,255,0.8); color: var(--text); }
                .nav-btn:active{ transform: translateY(1px); }
                .nav-btn.active{
                  background: #ffffff; color: var(--active);
                  box-shadow: 0 6px 18px rgba(15,23,42,0.08);
                }
                .container{ max-width: 1100px; margin: 0 auto; padding: 18px 16px; }

                .card{
                  border: 1px solid var(--border);
                  border-radius: 16px;
                  background: rgba(255,255,255,0.9);
                  box-shadow: 0 12px 30px rgba(15,23,42,0.06);
                  padding: 14px;
                }
                .row{ display:flex; gap: 10px; flex-wrap: wrap; }
                .input{
                  border: 1px solid var(--border);
                  border-radius: 12px;
                  padding: 10px 12px;
                  min-width: 240px;
                  outline: none;
                }
                .btn{
                  border: 1px solid var(--border);
                  border-radius: 12px;
                  padding: 10px 14px;
                  cursor: pointer;
                  background: #fff;
                  font-weight: 700;
                }
                .btn:disabled{ opacity: 0.6; cursor: not-allowed; }
                .muted{ color: var(--muted); }
                .error{
                  border: 1px solid #fecaca;
                  background: #fff1f2;
                  color: #9f1239;
                  padding: 10px 12px;
                  border-radius: 12px;
                }
                .ok{
                  border: 1px solid #bbf7d0;
                  background: #f0fdf4;
                  color: #14532d;
                  padding: 10px 12px;
                  border-radius: 12px;
                }
                "
            </style>

            <header class="appbar">
                <div class="appbar-inner">
                    <div class="brand" role="link" tabindex="0" on:click=go("/")>
                        <span class="logo">"🔎"</span>
                        <span>"Leptos Semantic"</span>
                    </div>

                    <nav class="nav" aria-label="Primary">
                        <button
                            type="button"
                            class=move || if is_search() { "nav-btn active" } else { "nav-btn" }
                            on:click=go("/")
                        >
                            "Search"
                        </button>

                        <button
                            type="button"
                            class=move || if is_create() { "nav-btn active" } else { "nav-btn" }
                            on:click=go("/create")
                        >
                            "Create"
                        </button>
                    </nav>
                </div>
            </header>

            <main class="container">
                <Routes fallback=|| view! { <p>"404 - Not Found"</p> }>
                    <Route path=path!() view=SemanticSearchPage />
                    <Route path=path!("create") view=SemanticCreatePage />
                </Routes>
            </main>
        </>
    }
}
