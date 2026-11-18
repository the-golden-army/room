use actix_web::{web, HttpRequest, HttpResponse};
use serde::Deserialize;

use crate::auth::{self, Auth};
use crate::boot;
use crate::models::*;
use crate::state::AppState;
use crate::store;
use crate::templates::{self, admin, calls, chat, files, account};

fn html(s: String) -> HttpResponse {
    HttpResponse::Ok().content_type("text/html; charset=utf-8").body(s)
}

async fn guard(req: &HttpRequest, state: &web::Data<AppState>) -> Result<Auth, HttpResponse> {
    match auth::resolve(req, state).await {
        Some(a) => Ok(a),
        None => Err(HttpResponse::Found()
            .append_header(("location", "/gate"))
            .finish()),
    }
}

#[derive(Deserialize)]
pub struct ChatQuery {
    pub conv: Option<i64>,
}

pub async fn chat_index(
    req: HttpRequest,
    state: web::Data<AppState>,
    q: web::Query<ChatQuery>,
) -> HttpResponse {
    chat_inner(req, state, None, q).await
}

pub async fn chat_item(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<i64>,
    q: web::Query<ChatQuery>,
) -> HttpResponse {
    chat_inner(req, state, Some(path.into_inner()), q).await
}

async fn chat_inner(
    req: HttpRequest,
    state: web::Data<AppState>,
    route_id: Option<i64>,
    q: web::Query<ChatQuery>,
) -> HttpResponse {
    let a = match guard(&req, &state).await {
        Ok(a) => a,
        Err(r) => return r,
    };
    let convs = store::list_conversations(&state.pool, a.user.id).await.unwrap_or_default();
    let active = route_id
        .or(q.conv)
        .or_else(|| convs.first().map(|c| c.id))
        .unwrap_or(0);
    let boot = boot::build(&state, &a, "chat").await;
    let conv = convs.iter().find(|c| c.id == active).cloned();
    let msgs = if active > 0 {
        store::list_messages(&state.pool, active, 0, state.cfg.page_size)
            .await
            .unwrap_or_default()
    } else {
        vec![]
    };
    let dto = crate::api::messages_dto(&state.pool, &msgs, a.user.id).await;
    let list = chat::list_column(&state, &boot, &convs, active);
    let main = chat::main_column(&boot, conv.as_ref(), &dto.unwrap_or_default(), &boot.users);
    let side = match conv {
        Some(c) => chat::details_panel(&boot, &c),
        None => String::new(),
    };
    let mobile_list = route_id.is_none() && q.conv.is_none();
    html(templates::page_opts(&boot, "chat", &list, &main, &side, "Chat", mobile_list))
}

#[derive(Deserialize)]
pub struct FilesQuery {
    pub parent: Option<i64>,
    pub trash: Option<i32>,
    pub select: Option<i64>,
}

pub async fn files_page(req: HttpRequest, state: web::Data<AppState>, q: web::Query<FilesQuery>) -> HttpResponse {
    let a = match guard(&req, &state).await {
        Ok(a) => a,
        Err(r) => return r,
    };
    let parent = q.parent.unwrap_or(0);
    let trashed = q.trash.unwrap_or(0) == 1;
    let mut path: Vec<FileDto> = Vec::new();
    let mut cur = parent;
    let mut guard_n = 0;
    while cur > 0 && guard_n < 40 {
        guard_n += 1;
        match store::get_file(&state.pool, cur).await.ok().flatten() {
            Some(f) if f.kind == "folder" => {
                let users = store::list_users(&state.pool).await.unwrap_or_default();
                let by_id: std::collections::HashMap<i64, User> =
                    users.into_iter().map(|u| (u.id, u)).collect();
                path.push(files::file_dto_pub(&f, &by_id, 0));
                cur = f.parent_id;
            }
            _ => break,
        }
    }
    path.reverse();
    let items: Vec<FileNode> = store::list_files(&state.pool, parent, trashed).await.unwrap_or_default();
    let users = store::list_users(&state.pool).await.unwrap_or_default();
    let by_id: std::collections::HashMap<i64, User> = users.into_iter().map(|u| (u.id, u)).collect();
    let mut dtos: Vec<FileDto> = Vec::new();
    for f in &items {
        let children = if f.kind == "folder" {
            store::child_count(&state.pool, f.id).await.unwrap_or(0)
        } else {
            0
        };
        dtos.push(files::file_dto_pub(f, &by_id, children));
    }
    let selected = match q.select {
        Some(id) => dtos.iter().find(|f| f.id == id).cloned(),
        None => None,
    };
    let boot = boot::build(&state, &a, "files").await;
    let used = store::storage_used(&state.pool).await.unwrap_or(0);
    let list = files::list_column(&state, &boot, used, 5 * 1024 * 1024 * 1024);
    let view = files::FileView {
        path,
        items: dtos.clone(),
        trashed,
        view: "list".into(),
    };
    let main = files::main_column(&boot, &view, selected.as_ref());
    let side = match selected {
        Some(f) => {
            let shares: Vec<(UserDto, String)> = vec![];
            files::details_panel(&f, &boot, &shares)
        }
        None => String::new(),
    };
    html(templates::page(&boot, "files", &list, &main, &side, "Files"))
}

pub async fn calls_page(req: HttpRequest, state: web::Data<AppState>) -> HttpResponse {
    let a = match guard(&req, &state).await {
        Ok(a) => a,
        Err(r) => return r,
    };
    let boot = boot::build(&state, &a, "calls").await;
    let convs = store::list_conversations(&state.pool, a.user.id).await.unwrap_or_default();
    let history_rows = store::call_history(&state.pool, 20).await.unwrap_or_default();
    let mut history = Vec::new();
    for c in &history_rows {
        let parts = store::call_participants(&state.pool, c.id).await.unwrap_or_default();
        history.push(crate::api::call_dto(c, &boot.users, &parts));
    }
    let active = match store::active_call(&state.pool).await.ok().flatten() {
        Some(c) => {
            let parts = store::call_participants(&state.pool, c.id).await.unwrap_or_default();
            Some(crate::api::call_dto(&c, &boot.users, &parts))
        }
        None => None,
    };
    let list = calls::list_column(&boot, &history, &convs, active.as_ref());
    let main = match active.as_ref() {
        Some(c) => {
            let tiles: Vec<calls::Tile> = c
                .participants
                .iter()
                .map(|uid| {
                    let u = boot.users.iter().find(|u| u.id == *uid);
                    calls::Tile {
                        user_id: *uid,
                        name: u.map(|u| u.name.clone()).unwrap_or_default(),
                        color: u.map(|u| u.color.clone()).unwrap_or_default(),
                        initials: u.map(|u| u.initials.clone()).unwrap_or_default(),
                        speaking: false,
                        video: c.kind == "video",
                        muted: false,
                        local: *uid == a.user.id,
                    }
                })
                .collect();
            calls::stage(&boot, c, &tiles, crate::util::now_ms() - c.started_at)
        }
        None => calls::lobby(&boot, &convs, "video"),
    };
    let side = match active.as_ref() {
        Some(c) => calls::details_panel(c, &boot.users),
        None => String::new(),
    };
    html(templates::page(&boot, "calls", &list, &main, &side, "Calls"))
}

#[derive(Deserialize)]
pub struct AdminQuery {
    pub tab: Option<String>,
}

pub async fn admin_page(
    req: HttpRequest,
    state: web::Data<AppState>,
    q: web::Query<AdminQuery>,
) -> HttpResponse {
    let a = match guard(&req, &state).await {
        Ok(a) => a,
        Err(r) => return r,
    };
    if a.user.role != "owner" && a.user.role != "admin" {
        return HttpResponse::Found().append_header(("location", "/")).finish();
    }
    let tab = q.tab.clone().unwrap_or_else(|| "overview".into());
    let tab = if admin::TABS.iter().any(|(k, _, _)| *k == tab) { tab } else { "overview".into() };
    let boot = boot::build(&state, &a, "admin").await;
    let list = admin::list_column(&boot, &tab, boot.pending_requests);
    let main = match tab.as_str() {
        "requests" => {
            let pending: Vec<DeviceDto> = boot
                .devices
                .iter()
                .filter(|d| !d.approved && !d.revoked)
                .cloned()
                .collect();
            admin::requests(&boot, &pending)
        }
        "devices" => admin::devices(&boot),
        "users" => admin::users(&boot, a.user.id),
        "audit" => {
            let entries: Vec<AuditDto> = store::list_audit(&state.pool, 300)
                .await
                .unwrap_or_default()
                .iter()
                .map(|e| AuditDto {
                    id: e.id,
                    actor_name: e.actor_name.clone(),
                    action: e.action.clone(),
                    entity: e.entity.clone(),
                    detail: e.detail.clone(),
                    ip: e.ip.clone(),
                    created_at: e.created_at,
                })
                .collect();
            admin::audit(&boot, &entries)
        }
        "backups" => {
            let items: Vec<BackupDto> = store::list_backups(&state.pool)
                .await
                .unwrap_or_default()
                .iter()
                .map(|b| BackupDto { id: b.id, name: b.name.clone(), size: b.size, created_at: b.created_at })
                .collect();
            admin::backups(&admin::BackupsView { items })
        }
        "config" => admin::config(&boot, ""),
        _ => {
            let stats = store::stats(&state.pool).await.unwrap_or_default();
            let audit: Vec<AuditDto> = store::list_audit(&state.pool, 9)
                .await
                .unwrap_or_default()
                .iter()
                .map(|e| AuditDto {
                    id: e.id,
                    actor_name: e.actor_name.clone(),
                    action: e.action.clone(),
                    entity: e.entity.clone(),
                    detail: e.detail.clone(),
                    ip: e.ip.clone(),
                    created_at: e.created_at,
                })
                .collect();
            admin::overview(&boot, &stats, &audit, "")
        }
    };
    html(templates::page(&boot, "admin", &list, &main, "", "Administration"))
}

#[derive(Deserialize)]
pub struct AccountQuery {
    pub tab: Option<String>,
}

pub async fn account_page(
    req: HttpRequest,
    state: web::Data<AppState>,
    q: web::Query<AccountQuery>,
) -> HttpResponse {
    let a = match guard(&req, &state).await {
        Ok(a) => a,
        Err(r) => return r,
    };
    let tab = q.tab.clone().unwrap_or_else(|| "profile".into());
    let tab = match tab.as_str() {
        "devices" | "security" => tab,
        _ => "profile".into(),
    };
    let boot = boot::build(&state, &a, "account").await;
    let list = account::list_column(&boot);
    let main = account::main_column(&boot, &tab);
    html(templates::page(&boot, "account", &list, &main, "", "Account"))
}

pub async fn gate(req: HttpRequest, state: web::Data<AppState>) -> HttpResponse {
    if auth::resolve(&req, &state).await.is_some() {
        return HttpResponse::Found().append_header(("location", "/")).finish();
    }
    // Somebody who already asked for access must not be asked again: send them
    // to their own waiting screen. This is what used to happen on refresh.
    if let Some((_, user)) = auth::pending_device(&req, &state).await {
        return HttpResponse::Found()
            .append_header(("location", "/waiting"))
            .finish();
    }
    // `resolve` rejects suspended accounts, which would otherwise loop forever:
    // the gate would offer "request access", registration would answer "approved",
    // and the workspace would bounce straight back here. Show the real reason.
    if auth::suspended_user(&req, &state).await.is_some() {
        return html(templates::gate::suspended_page(&state));
    }
    html(templates::gate::gate_page(&state, None).await)
}

/// The "you are waiting" screen. It lives on its own path so the browser can
/// refresh it without re-posting the form, and so the visitor can bookmark it.
pub async fn waiting(req: HttpRequest, state: web::Data<AppState>) -> HttpResponse {
    if auth::resolve(&req, &state).await.is_some() {
        return HttpResponse::Found().append_header(("location", "/")).finish();
    }
    if let Some((device, user)) = auth::pending_device(&req, &state).await {
        return html(templates::gate::waiting_page(&state, &user, &device.name).await);
    }
    // No pending credential (approved, revoked, or never asked): back to the gate
    // so the reason is explained rather than showing a stale spinner.
    HttpResponse::Found().append_header(("location", "/gate")).finish()
}

pub async fn health(state: web::Data<AppState>) -> HttpResponse {
    let db = sqlx::query("SELECT 1").execute(&state.pool).await.is_ok();
    HttpResponse::Ok().json(serde_json::json!({
        "ok": true,
        "name": state.cfg.server_name,
        "version": state.version,
        "db": db,
        "uptime_ms": state.uptime_ms(),
    }))
}

pub async fn server_info(state: web::Data<AppState>) -> HttpResponse {
    HttpResponse::Ok().json(crate::api::server_info(&state))
}
