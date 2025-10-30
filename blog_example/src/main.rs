mod db;
mod handlers;
mod models;

use axum::{
    routing::{get, post},
    Router,
};
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let pool = db::init_db().await.expect("Failed to initialize database");

    let app = Router::new()
        .route("/", get(handlers::index))
        .route("/posts/new", get(handlers::new_post_form))
        .route("/posts", post(handlers::create_post))
        .route("/posts/:id", get(handlers::show_post))
        .route("/posts/:id/edit", get(handlers::edit_post_form))
        .route("/posts/:id/update", post(handlers::update_post))
        .route("/posts/:id/delete", post(handlers::delete_post))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    println!("Blog server running on http://127.0.0.1:3000");

    axum::serve(listener, app).await.unwrap();
}
