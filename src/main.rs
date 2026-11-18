mod api;
mod auth;
mod boot;
mod config;
mod db;
mod models;
mod pages;
mod realtime;
mod state;
mod store;
mod templates;
mod util;

use actix::Actor;
use actix_web::{web, App, HttpResponse, HttpServer};
use state::AppState;

const APP_CSS: &str = include_str!("../assets/app.dist.css");
const APP_JS: &str = include_str!("../assets/app.js");

const FAVICON: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32"><defs><linearGradient id="g" x1="0" y1="0" x2="32" y2="32" gradientUnits="userSpaceOnUse"><stop stop-color="#6d4dff"/><stop offset="1" stop-color="#4a2ae0"/></linearGradient></defs><rect width="32" height="32" rx="9" fill="url(#g)"/><path d="M9 22.5v-9.2a3.3 3.3 0 0 1 3.3-3.3h7.4a3.3 3.3 0 0 1 3.3 3.3v4.6a3.3 3.3 0 0 1-3.3 3.3h-6.4l-4.3 3.6v-2.3z" fill="#fff"/><circle cx="13.4" cy="14.4" r="1.15" fill="#5333f4"/><circle cx="16.5" cy="14.4" r="1.15" fill="#5333f4"/><circle cx="19.6" cy="14.4" r="1.15" fill="#f6960c"/></svg>"##;

pub async fn asset_css() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/css; charset=utf-8")
        .insert_header(("cache-control", "public, max-age=31536000, immutable"))
        .body(APP_CSS)
}

pub async fn asset_js() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/javascript; charset=utf-8")
        .insert_header(("cache-control", "public, max-age=31536000, immutable"))
        .body(APP_JS)
}

pub async fn favicon() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("image/svg+xml")
        .insert_header(("cache-control", "public, max-age=86400"))
        .body(FAVICON)
}

/// Resolve the code that lets the host in when nobody is approved yet.
///
/// Returns the code plus whether it was generated just now (so it is worth
/// printing loudly), and writes it into `cfg` for the request handlers.
async fn resolve_setup_code(
    pool: &sqlx::SqlitePool,
    cfg: &mut config::Config,
) -> Result<Option<(String, bool)>, sqlx::Error> {
    use crate::util::pairing_code;

    // An explicit env var wins and is not written to the database, but it is
    // still the active code, so mirror it into settings for the admin page to
    // display. (It is re-derived from the env var on every start.)
    if !cfg.setup_code.is_empty() {
        let code = cfg.setup_code.clone();
        let _ = db::set_setting(pool, "setup_code", &code).await;
        return Ok(Some((code, false)));
    }
    if let Some(saved) = db::setting(pool, "setup_code").await {
        let saved = saved.trim().to_string();
        if !saved.is_empty() {
            cfg.setup_code = saved.clone();
            return Ok(Some((saved, false)));
        }
    }
    let code = pairing_code();
    db::set_setting(pool, "setup_code", &code).await?;
    cfg.setup_code = code.clone();
    Ok(Some((code, true)))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let mut cfg = config::Config::from_env();
    let pool = db::connect(&cfg).await.expect("database init failed");
    db::migrate(&pool).await.expect("migration failed");
    seed(&pool).await;

    // Resolve the host's setup code before anything can serve a request.
    // Priority: explicit ROOM_SETUP_CODE, then the code stored from first run,
    // then a freshly generated one persisted for future restarts.
    let boot_notice = match resolve_setup_code(&pool, &mut cfg).await {
        Ok(n) => n,
        Err(e) => {
            eprintln!("warning: could not persist the setup code: {e}");
            None
        }
    };

    let hub = realtime::Hub::new().start();
    let state = web::Data::new(AppState::new(pool, hub, cfg));
    let bind = state.cfg.bind.clone();

    // Presence reaper: mark long-silent sockets offline.
    {
        let st = state.clone();
        actix_web::rt::spawn(async move {
            let mut tick = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                tick.tick().await;
                store::reaper(&st.pool).await;
                let snap = store::presence_snapshot(&st.pool).await;
                st.broadcast(serde_json::json!({"t":"presence","users":snap.online}));
            }
        });
    }

    println!(
        "ROOM {} listening on http://{} (data: {}, files: {})",
        state.version,
        bind,
        state.cfg.data_dir.display(),
        state.cfg.files_dir.display()
    );
    // The whole point of ROOM is reaching it from other devices; print the
    // address a phone on the same network should open.
    match state.cfg.lan_url() {
        Some(url) => println!("   open on your network: {url}"),
        None => println!(
            "   open on your network: http://<this-computer's-LAN-IP>:{} (no LAN address detected)",
            state.cfg.port().unwrap_or(8080)
        ),
    }
    if let Some((code, fresh)) = boot_notice {
        if fresh {
            // Generated on this run: show it prominently, it is the only way in.
            println!();
            println!("   +--------------------------------------------+");
            println!("   |  Setup code: {:<30}|", code);
            println!("   +--------------------------------------------+");
            println!("   Type this on the first device you connect, or any time");
            println!("   the owner has no approved device. It never expires.");
        } else {
            println!("   setup code: {code}  (host override, ROOM_SETUP_CODE)");
        }
    }

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .wrap(actix_web::middleware::Compress::default())
            .wrap(actix_web::middleware::NormalizePath::trim())
            .route("/api/health", web::get().to(pages::health))
            .route("/api/server-info", web::get().to(pages::server_info))
            .route("/assets/app.css", web::get().to(asset_css))
            .route("/assets/app.js", web::get().to(asset_js))
            .route("/favicon.svg", web::get().to(favicon))
            .route("/ws", web::get().to(realtime::ws_index))
            .route("/gate", web::get().to(pages::gate))
            .route("/waiting", web::get().to(pages::waiting))
            .route("/api/request-access", web::post().to(auth::request_access))
            .route("/api/logout", web::get().to(auth::logout))
            .route("/", web::get().to(pages::chat_index))
            .route("/chat/{id}", web::get().to(pages::chat_item))
            .route("/files", web::get().to(pages::files_page))
            .route("/calls", web::get().to(pages::calls_page))
            .route("/admin", web::get().to(pages::admin_page))
            .route("/account", web::get().to(pages::account_page))
            // chat api
            .route("/api/conversations", web::get().to(api::list_conversations))
            .route("/api/conversations", web::post().to(api::create_conversation))
            .route("/api/conversations/{id}", web::get().to(api::conversation_detail))
            .route("/api/conversations/{id}", web::patch().to(api::patch_conversation))
            .route("/api/conversations/{id}/messages", web::get().to(api::list_messages))
            .route("/api/conversations/{id}/messages", web::post().to(api::send_message))
            .route("/api/conversations/{id}/read", web::post().to(api::mark_read))
            .route("/api/conversations/{id}/mute", web::post().to(api::mute_conversation))
            .route("/api/messages/{id}", web::patch().to(api::edit_message))
            .route("/api/messages/{id}", web::delete().to(api::delete_message))
            .route("/api/messages/{id}/reactions", web::post().to(api::react_message))
            .route("/api/search", web::get().to(api::search))
            .route("/api/users", web::get().to(api::list_users))
            .route("/api/users/me", web::patch().to(api::update_profile))
            // files api
            .route("/api/files", web::get().to(api::browse_files))
            .route("/api/files", web::post().to(api::create_folder))
            .route("/api/files/upload", web::post().to(api::upload_file))
            .route("/api/files/{id}", web::get().to(api::file_detail))
            .route("/api/files/{id}", web::patch().to(api::patch_file))
            .route("/api/files/{id}", web::delete().to(api::trash_file))
            .route("/api/files/{id}/restore", web::post().to(api::restore_file))
            .route("/api/files/{id}/purge", web::delete().to(api::purge_file))
            .route("/api/files/{id}/raw", web::get().to(api::file_raw))
            .route("/api/files/{id}/download", web::get().to(api::file_download))
            // calls api
            .route("/api/calls", web::get().to(api::calls_index))
            .route("/api/calls", web::post().to(api::calls_start))
            .route("/api/calls/{id}/join", web::post().to(api::calls_join))
            .route("/api/calls/{id}/leave", web::post().to(api::calls_leave))
            .route("/api/calls/{id}/end", web::post().to(api::calls_end))
            // admin api
            .route("/api/admin/overview", web::get().to(api::admin_overview))
            .route("/api/admin/requests", web::get().to(api::admin_requests))
            .route("/api/admin/requests/{id}/approve", web::post().to(api::admin_approve))
            .route("/api/admin/requests/{id}/reject", web::post().to(api::admin_reject))
            .route("/api/admin/devices/{id}/revoke", web::post().to(api::admin_revoke))
            .route("/api/admin/users/{id}", web::patch().to(api::admin_update_user))
            .route("/api/admin/audit", web::get().to(api::admin_audit))
            .route("/api/admin/backups", web::get().to(api::admin_backups))
            .route("/api/admin/backups", web::post().to(api::admin_create_backup))
            .route("/api/admin/backups/{id}", web::delete().to(api::admin_delete_backup))
            .route("/api/admin/config", web::post().to(api::admin_config))
            .route("/api/admin/shutdown", web::post().to(api::admin_shutdown))
            .route("/api/admin/restart", web::post().to(api::admin_restart))
            .default_service(web::to(pages::gate))
    })
    .bind(bind.as_str())?
    .run()
    .await
}

/// Create the first owner account when the server is brand new so the host can
/// sign in and start approving devices.
async fn seed(pool: &sqlx::SqlitePool) {
    let owner = store::owner_count(pool).await.unwrap_or(0);
    if owner == 0 {
        // No owner yet: nothing to create. The first connection request becomes
        // the owner (see auth::request_access).
    }
    let _ = owner;
    let theme = db::setting(pool, "theme").await.unwrap_or_default();
    if theme.is_empty() {
        let _ = db::set_setting(pool, "theme", "light").await;
    }
    if db::setting(pool, "server_name").await.is_none() {
        let _ = db::set_setting(pool, "server_name", "ROOM").await;
    }
    if db::setting(pool, "max_upload_mb").await.is_none() {
        let _ = db::set_setting(pool, "max_upload_mb", "2048").await;
    }
}
