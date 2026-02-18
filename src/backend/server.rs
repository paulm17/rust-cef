use axum::{routing::get, Router};
use axum::response::Html;
use tokio::net::TcpListener;

pub async fn start_server() {
    let app = Router::new().route("/", get(|| async { 
        Html("<h1>Hello from Rust Backend!</h1>") 
    }));

    let listener = match TcpListener::bind("0.0.0.0:3000").await {
        Ok(l) => {
            println!("Backend running on localhost:3000");
            l
        },
        Err(_) => return,
    };
    
    axum::serve(listener, app).await.unwrap();
}
