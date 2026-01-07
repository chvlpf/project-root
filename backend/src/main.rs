mod config;
mod flat_index;
mod handler;
mod model;
mod presenter;
mod utils;

use axum::{routing::post, Router};
use tokio::net::TcpListener;

use crate::config::load_config;
use crate::flat_index::FlatIndex;
use crate::handler::{create_data, get_data};

use std::sync::Arc;
use tokio::sync::Mutex;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

#[derive(Clone)]
pub struct AppState {
    pub embedder: Arc<Mutex<TextEmbedding>>,
    pub index: Arc<Mutex<FlatIndex>>,
}

#[tokio::main]
async fn main() {
    // ---- config ----
    let config = load_config();
    let addr = format!("{}:{}", config.app.url, config.app.port);

    // ---- init embedder (fastembed) ----
    let mut opts = InitOptions::default();
    opts.model_name = EmbeddingModel::AllMiniLML6V2;

    let embedder = TextEmbedding::try_new(opts).expect("failed to init TextEmbedding (fastembed)");

    let dim = 384usize;
    let index = FlatIndex::open_or_create("src/data/reviews.index", dim)
        .expect("failed to open/create FlatIndex");

    let state = Arc::new(AppState {
        embedder: Arc::new(Mutex::new(embedder)),
        index: Arc::new(Mutex::new(index)),
    });

    // ---- cors + middleware ----
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    let middleware_stack = ServiceBuilder::new().layer(cors);

    // ---- routes ----
    let app = Router::new()
        .route("/create-data", post(create_data))
        .route("/get-data", post(get_data))
        .with_state(state)
        .layer(middleware_stack);

    println!("\n");
    println!("--------------------------------------");
    println!("running on http://{}", addr);
    println!("--------------------------------------\n");

    let listener = TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
