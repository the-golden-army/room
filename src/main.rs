use actix_web::{web, App, HttpResponse, HttpServer};
use sqlx::sqlite::SqlitePoolOptions;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::query("CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)")
        .execute(&pool)
        .await
        .unwrap();
    let _ = actix_web_actors::ws::Message::Text("x".into());
    let _ = actix_multipart::Multipart::new;
    let _ = actix_files::Files::new("/", "./assets");
    let _ = mime_guess::from_path("a.png").first();
    let _ = uuid::Uuid::new_v4();
    let _ = chrono::Utc::now();
    let _ = rand::random::<u64>();
    let _ = sha2::Sha256::digest(b"x");
    let _ = hex::encode(b"x");
    let _ = percent_encoding::percent_decode_str("a");
    let _ = futures_util::future::ready(1);
    let _ = tokio::time::Duration::from_secs(1);
    HttpServer::new(|| App::new().route("/health", web::get().to(|| async { HttpResponse::Ok().body("ok") })))
        .bind(("0.0.0.0", 8080))?
        .run()
        .await
}
