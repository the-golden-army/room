use actix::prelude::*;
use actix_files::NamedFile;
use actix_web::http::header::{HeaderValue, CONTENT_DISPOSITION};
use actix_web::{web, HttpRequest, HttpResponse, ResponseError};
use futures_util::StreamExt;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::fmt;

use crate::auth::{self, Auth};
use crate::models::*;
use crate::realtime::Hub;
use crate::state::AppState;
use crate::store;
use crate::util::{clamp_len, clean_node_name, client_ip, human_bytes, new_id, sanitize_filename};
pub struct ApiError {
    pub status: u16,
    pub msg: String,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}
impl fmt::Debug for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ApiError({}): {}", self.status, self.msg)
    }
}

impl ResponseError for ApiError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        actix_web::http::StatusCode::from_u16(self.status).unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR)
    }
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).json(json!({"error": self.msg}))
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        ApiError { status: 500, msg: format!("database error: {e}") }
    }
}

pub type ApiResult = Result<HttpResponse, ApiError>;

fn err(status: u16, msg: &str) -> ApiError {
    ApiError { status, msg: msg.to_string() }
}

async fn require(req: &HttpRequest, state: &AppState) -> Result<Auth, ApiError> {
    auth::resolve(req, state)
        .await
        .ok_or_else(|| err(401, "not authorised"))
}

async fn require_admin(req: &HttpRequest, state: &AppState) -> Result<Auth, ApiError> {
    let a = require(req, state).await?;
    if a.user.role != "owner" && a.user.role != "admin" {
        return Err(err(403, "administrator role required"));
    }
    Ok(a)
}

fn j<T: serde::Serialize>(v: &T) -> ApiResult {
    Ok(HttpResponse::Ok().json(v))
}

fn jval(v: Value) -> ApiResult {
    Ok(HttpResponse::Ok().json(v))
}

// ------------------------------------------------------------------ helpers

async fn users_map(pool: &SqlitePool) -> std::collections::HashMap<i64, User> {
    store::list_users(pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|u| (u.id, u))
        .collect()
}

/// Build MsgDto for a batch of messages (single round trip for users + reactions).
pub async fn messages_dto(
    pool: &SqlitePool,
    msgs: &[Message],
    me: i64,
) -> Result<Vec<MsgDto>, sqlx::Error> {
    let users = users_map(pool).await;
    let ids: Vec<i64> = msgs.iter().map(|m| m.id).collect();
    let reacts = store::reactions_for(pool, &ids).await.unwrap_or_default();
    let mut reply_ids: Vec<i64> = msgs.iter().filter(|m| m.reply_to_id > 0).map(|m| m.reply_to_id).collect();
    reply_ids.sort_unstable();
    reply_ids.dedup();
    let replies = store::reply_index(pool, &reply_ids).await.unwrap_or_default();
    Ok(msgs
        .iter()
        .map(|m| {
            let r: Vec<ReactionRow> = reacts.iter().filter(|r| r.message_id == m.id).cloned().collect();
            store::message_dto(m, &users, r, me, &replies)
        })
        .collect())
}

fn broadcast_msg(state: &AppState, m: &MsgDto, members: &[i64]) {
    let payload = json!({
        "t": "message:new",
        "conversation_id": m.conversation_id,
        "message": m,
        "last_preview": m.body,
        "last_sender": m.sender_name,
        "last_message_at": m.created_at,
    });
    state.to_users(members.to_vec(), payload);
}

// ------------------------------------------------------------------ chat

pub async fn list_conversations(
    req: HttpRequest,
    state: web::Data<AppState>,
) -> ApiResult {
    let a = require(&req, &state).await?;
    let convs = store::list_conversations(&state.pool, a.user.id).await?;
    j(&json!({"conversations": convs}))
}

#[derive(Deserialize)]
pub struct ConvForm {
    pub kind: Option<String>,
    pub user_id: Option<i64>,
    pub name: Option<String>,
    pub topic: Option<String>,
    pub members: Option<Vec<i64>>,
}

pub async fn create_conversation(
    req: HttpRequest,
    state: web::Data<AppState>,
    form: web::Json<ConvForm>,
) -> ApiResult {
    let a = require(&req, &state).await?;
    let f = form.into_inner();
    let conv = match f.kind.as_deref() {
        Some("group") => {
            let name = clean_node_name(f.name.as_deref().unwrap_or(""));
            if name == "Untitled" {
                return Err(err(400, "Group name is required"));
            }
            let mut members = f.members.unwrap_or_default();
            members.retain(|id| *id != a.user.id);
            store::create_group(
                &state.pool,
                &name,
                f.topic.as_deref().unwrap_or(""),
                a.user.id,
                &members,
            )
            .await?
        }
        _ => {
            let other = f.user_id.ok_or_else(|| err(400, "user_id is required"))?;
            if other == a.user.id {
                return Err(err(400, "Cannot start a conversation with yourself"));
            }
            store::create_direct(&state.pool, a.user.id, other).await?
        }
    };
    let dto = store::list_conversations(&state.pool, a.user.id)
        .await?
        .into_iter()
        .find(|c| c.id == conv.id)
        .ok_or_else(|| err(500, "conversation created but not found"))?;
    state.broadcast(json!({"t": "conversations:changed", "user_id": a.user.id}));
    let _ = state.audit(
        a.user.id,
        &a.user.name,
        "conversation.create",
        "conversation",
        conv.id,
        &conv.name,
        &client_ip(&req),
    ).await;
    j(&json!({"conversation": dto}))
}

pub async fn conversation_detail(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> ApiResult {
    let a = require(&req, &state).await?;
    let id = path.into_inner();
    if !store::is_member(&state.pool, id, a.user.id).await? {
        return Err(err(403, "not a member"));
    }
    let conv = store::get_conversation(&state.pool, id).await?;
    let members = store::conversation_members(&state.pool, id).await?;
    let users = store::list_users(&state.pool).await.unwrap_or_default();
    let by_id: std::collections::HashMap<i64, User> = users.into_iter().map(|u| (u.id, u)).collect();
    let mut member_dtos = Vec::new();
    for m in &members {
        if let Some(u) = by_id.get(&m.user_id) {
            member_dtos.push(UserDto::from(u));
        }
    }
    let conv_dto = match conv {
        Some(c) => {
            let list = store::list_conversations(&state.pool, a.user.id).await?;
            list.into_iter().find(|x| x.id == id)
        }
        None => None,
    };
    j(&json!({
        "conversation": conv_dto,
        "members": member_dtos,
    }))
}

#[derive(Deserialize)]
pub struct MsgQuery {
    pub before: Option<i64>,
    pub limit: Option<i64>,
}

pub async fn list_messages(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<i64>,
    q: web::Query<MsgQuery>,
) -> ApiResult {
    let a = require(&req, &state).await?;
    let cid = path.into_inner();
    if !store::is_member(&state.pool, cid, a.user.id).await? {
        return Err(err(403, "not a member"));
    }
    let limit = clamp_len(q.limit.unwrap_or(0), 1, 200, state.cfg.page_size);
    let before = q.before.unwrap_or(0);
    let msgs = store::list_messages(&state.pool, cid, before, limit).await?;
    let total = store::message_count(&state.pool, cid).await?;
    let dto = messages_dto(&state.pool, &msgs, a.user.id).await?;
    let oldest = msgs.first().map(|m| m.id).unwrap_or(0);
    j(&json!({
        "messages": dto,
        "has_more": oldest > 1 && (total as i64) > dto.len() as i64 && before != oldest,
        "oldest": oldest,
        "total": total,
    }))
}

#[derive(Deserialize)]
pub struct SendForm {
    pub body: String,
    pub reply_to_id: Option<i64>,
}

pub async fn send_message(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<i64>,
    form: web::Json<SendForm>,
) -> ApiResult {
    let a = require(&req, &state).await?;
    let cid = path.into_inner();
    if !store::is_member(&state.pool, cid, a.user.id).await? {
        return Err(err(403, "not a member"));
    }
    let body = form.body.clone();
    if body.trim().is_empty() {
        return Err(err(400, "message is empty"));
    }
    if body.chars().count() > 8000 {
        return Err(err(400, "message is too long"));
    }
    let reply_to_id = form.reply_to_id.unwrap_or(0);
    let msg = store::insert_message(
        &state.pool,
        &store::NewMessage {
            conversation_id: cid,
            sender_id: a.user.id,
            body,
            reply_to_id,
        },
    )
    .await?;
    let dto = messages_dto(&state.pool, std::slice::from_ref(&msg), a.user.id).await?;
    let dto = dto.into_iter().next().unwrap();
    let members: Vec<i64> = store::conversation_members(&state.pool, cid)
        .await?
        .into_iter()
        .map(|m| m.user_id)
        .collect();
    broadcast_msg(&state, &dto, &members);
    let _ = state.audit(
        a.user.id,
        &a.user.name,
        "message.send",
        "conversation",
        cid,
        &format!("{} chars", dto.body.chars().count()),
        &client_ip(&req),
    ).await;
    j(&json!({"message": dto}))
}

#[derive(Deserialize)]
pub struct EditForm {
    pub body: String,
}

pub async fn edit_message(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<i64>,
    form: web::Json<EditForm>,
) -> ApiResult {
    let a = require(&req, &state).await?;
    let id = path.into_inner();
    let existing = store::get_message(&state.pool, id).await?;
    let m = existing.ok_or_else(|| err(404, "message not found"))?;
    if m.sender_id != Some(a.user.id) {
        return Err(err(403, "you can only edit your own messages"));
    }
    if m.deleted_at > 0 {
        return Err(err(400, "message was deleted"));
    }
    let body = form.body.clone();
    if body.trim().is_empty() {
        return Err(err(400, "message is empty"));
    }
    let updated = store::edit_message(&state.pool, id, &body).await?;
    let dto = match updated {
        Some(u) => messages_dto(&state.pool, std::slice::from_ref(&u), a.user.id).await?,
        None => return Err(err(404, "message not found")),
    };
    let dto = dto.into_iter().next().unwrap();
    let members: Vec<i64> = store::conversation_members(&state.pool, m.conversation_id)
        .await?
        .into_iter()
        .map(|mm| mm.user_id)
        .collect();
    state.to_users(members, json!({"t":"message:updated","message":dto}));
    j(&json!({"message": dto}))
}

pub async fn delete_message(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> ApiResult {
    let a = require(&req, &state).await?;
    let id = path.into_inner();
    let existing = store::get_message(&state.pool, id).await?;
    let m = existing.ok_or_else(|| err(404, "message not found"))?;
    let is_authorised = m.sender_id == Some(a.user.id) || a.user.role == "owner" || a.user.role == "admin";
    if !is_authorised {
        return Err(err(403, "not allowed"));
    }
    store::delete_message(&state.pool, id).await?;
    let members: Vec<i64> = store::conversation_members(&state.pool, m.conversation_id)
        .await?
        .into_iter()
        .map(|mm| mm.user_id)
        .collect();
    state.to_users(members, json!({"t":"message:deleted","message_id":id,"conversation_id":m.conversation_id}));
    j(&json!({"ok": true}))
}

#[derive(Deserialize)]
pub struct ReactForm {
    pub emoji: String,
    pub on: Option<bool>,
}

pub async fn react_message(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<i64>,
    form: web::Json<ReactForm>,
) -> ApiResult {
    let a = require(&req, &state).await?;
    let id = path.into_inner();
    let m = store::get_message(&state.pool, id).await?
        .ok_or_else(|| err(404, "message not found"))?;
    if !store::is_member(&state.pool, m.conversation_id, a.user.id).await? {
        return Err(err(403, "not a member"));
    }
    let emoji = form.emoji.trim().to_string();
    if emoji.is_empty() || emoji.chars().count() > 8 {
        return Err(err(400, "invalid emoji"));
    }
    let existing = store::reactions_for(&state.pool, &[id]).await?;
    let already = existing.iter().any(|r| r.user_id == a.user.id && r.emoji == emoji);
    let on = form.on.unwrap_or(!already);
    store::set_reaction(&state.pool, id, a.user.id, &emoji, on).await?;
    let msg = store::get_message(&state.pool, id).await?.unwrap();
    let dto = messages_dto(&state.pool, std::slice::from_ref(&msg), a.user.id).await?;
    let dto = dto.into_iter().next().unwrap();
    let members: Vec<i64> = store::conversation_members(&state.pool, m.conversation_id)
        .await?
        .into_iter()
        .map(|mm| mm.user_id)
        .collect();
    state.to_users(members, json!({"t":"message:updated","message":dto}));
    j(&json!({"message": dto}))
}

pub async fn mark_read(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> ApiResult {
    let a = require(&req, &state).await?;
    let cid = path.into_inner();
    store::mark_read(&state.pool, cid, a.user.id).await?;
    let convs = store::list_conversations(&state.pool, a.user.id).await?;
    if let Some(c) = convs.iter().find(|c| c.id == cid) {
        state.to_user(a.user.id, json!({"t":"conversation:read","conversation":c}));
    }
    j(&json!({"ok": true}))
}

#[derive(Deserialize)]
pub struct MuteForm {
    pub muted: bool,
}

pub async fn mute_conversation(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<i64>,
    form: web::Json<MuteForm>,
) -> ApiResult {
    let a = require(&req, &state).await?;
    let cid = path.into_inner();
    store::set_muted(&state.pool, cid, a.user.id, form.into_inner().muted).await?;
    j(&json!({"ok": true}))
}

#[derive(Deserialize)]
pub struct ConvPatch {
    pub name: Option<String>,
    pub topic: Option<String>,
}

pub async fn patch_conversation(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<i64>,
    form: web::Json<ConvPatch>,
) -> ApiResult {
    let a = require(&req, &state).await?;
    let cid = path.into_inner();
    let conv = store::get_conversation(&state.pool, cid).await?
        .ok_or_else(|| err(404, "conversation not found"))?;
    if conv.kind != "group" {
        return Err(err(400, "only group details are editable"));
    }
    let f = form.into_inner();
    let name = match f.name {
        Some(n) if !n.trim().is_empty() => clean_node_name(&n),
        _ => conv.name.clone(),
    };
    let topic = f.topic.unwrap_or_else(|| conv.topic.clone());
    store::update_conversation(&state.pool, cid, &name, &topic).await?;
    let members: Vec<i64> = store::conversation_members(&state.pool, cid)
        .await?
        .into_iter()
        .map(|m| m.user_id)
        .collect();
    state.to_users(members, json!({"t":"conversations:changed"}));
    j(&json!({"ok": true}))
}

// ------------------------------------------------------------------ search

pub async fn search(
    req: HttpRequest,
    state: web::Data<AppState>,
    q: web::Query<SearchQuery>,
) -> ApiResult {
    let a = require(&req, &state).await?;
    let term = q.q.trim().to_string();
    if term.chars().count() < 2 {
        return j(&json!({"messages": [], "files": [], "conversations": []}));
    }
    let pat = format!("%{}%", term.replace('%', "\\%").replace('_', "\\_"));
    let msgs: Vec<Message> = sqlx::query_as::<_, Message>(
        "SELECT m.* FROM messages m JOIN conversation_members cm ON cm.conversation_id = m.conversation_id
         WHERE cm.user_id = ? AND m.deleted_at = 0 AND m.body LIKE ? ESCAPE '\\'
         ORDER BY m.id DESC LIMIT 25",
    )
    .bind(a.user.id)
    .bind(&pat)
    .fetch_all(&state.pool)
    .await?;
    let dto = messages_dto(&state.pool, &msgs, a.user.id).await?;
    let files = store::search_files(&state.pool, &term, 25).await?;
    let users = store::list_users(&state.pool).await.unwrap_or_default();
    let by_id: std::collections::HashMap<i64, User> = users.into_iter().map(|u| (u.id, u)).collect();
    let convs = store::list_conversations(&state.pool, a.user.id).await?;
    let conv_hits: Vec<ConvDto> = convs
        .into_iter()
        .filter(|c| c.title.to_lowercase().contains(&term.to_lowercase()))
        .take(10)
        .collect();
    let file_dtos: Vec<FileDto> = files
        .iter()
        .map(|f| FileDto {
            id: f.id,
            parent_id: f.parent_id,
            name: f.name.clone(),
            mime: f.mime.clone(),
            size: f.size,
            kind: f.kind.clone(),
            trashed: f.trashed == 1,
            owner_id: f.owner_id.unwrap_or(0),
            owner_name: f
                .owner_id
                .and_then(|id| by_id.get(&id))
                .map(|u| u.name.clone())
                .unwrap_or_default(),
            shared: false,
            children: 0,
            created_at: f.created_at,
            updated_at: f.updated_at,
        })
        .collect();
    j(&json!({"messages": dto, "files": file_dtos, "conversations": conv_hits}))
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

// ------------------------------------------------------------------ files

pub fn file_dto(f: &FileNode, by_id: &std::collections::HashMap<i64, User>, shared: bool, children: i64) -> FileDto {
    FileDto {
        id: f.id,
        parent_id: f.parent_id,
        name: f.name.clone(),
        mime: f.mime.clone(),
        size: f.size,
        kind: f.kind.clone(),
        trashed: f.trashed == 1,
        owner_id: f.owner_id.unwrap_or(0),
        owner_name: f
            .owner_id
            .and_then(|id| by_id.get(id))
            .map(|u| u.name.clone())
            .unwrap_or_default(),
        shared,
        children,
        created_at: f.created_at,
        updated_at: f.updated_at,
    }
}

pub async fn browse_files(
    req: HttpRequest,
    state: web::Data<AppState>,
    q: web::Query<BrowseQuery>,
) -> ApiResult {
    let a = require(&req, &state).await?;
    let parent = q.parent.unwrap_or(0);
    let trashed = q.trash.unwrap_or(0) == 1;
    let mut path = Vec::new();
    let mut cur = parent;
    let mut guard = 0;
    while cur > 0 && guard < 40 {
        guard += 1;
        match store::get_file(&state.pool, cur).await? {
            Some(f) if f.kind == "folder" => {
                path.push(f.clone());
                cur = f.parent_id;
            }
            _ => break,
        }
    }
    path.reverse();
    let items = store::list_files(&state.pool, parent, trashed).await?;
    let users = store::list_users(&state.pool).await.unwrap_or_default();
    let by_id: std::collections::HashMap<i64, User> = users.into_iter().map(|u| (u.id, u)).collect();
    let ids: Vec<i64> = items.iter().map(|f| f.id).collect();
    let shares = store::share_counts(&state.pool, &ids).await.unwrap_or_default();
    let mut dtos = Vec::new();
    for f in &items {
        let children = if f.kind == "folder" {
            store::child_count(&state.pool, f.id).await.unwrap_or(0)
        } else {
            0
        };
        dtos.push(file_dto(f, &by_id, shares.get(&f.id).copied().unwrap_or(0) > 0, children));
    }
    j(&json!({"items": dtos, "path": path, "parent": parent, "trash": trashed}))
}

#[derive(Deserialize)]
pub struct BrowseQuery {
    pub parent: Option<i64>,
    pub trash: Option<i32>,
}

#[derive(Deserialize)]
pub struct UploadQuery {
    pub parent: Option<i64>,
    pub name: Option<String>,
}

pub async fn upload_file(
    req: HttpRequest,
    state: web::Data<AppState>,
    q: web::Query<UploadQuery>,
    mut payload: web::Payload,
) -> ApiResult {
    let a = require(&req, &state).await?;
    let parent = q.parent.unwrap_or(0);
    if parent > 0 {
        match store::get_file(&state.pool, parent).await? {
            Some(f) if f.kind == "folder" => {}
            _ => return Err(err(400, "parent folder not found")),
        }
    }
    let name = sanitize_filename(q.name.as_deref().unwrap_or(""));
    if name == "unnamed" {
        return Err(err(400, "file name is required"));
    }
    let key = new_id();
    let tmp = state.cfg.files_dir.join(format!(".tmp-{key}"));
    let mut size: u64 = 0;
    {
        let mut f = tokio::fs::File::create(&tmp).await.map_err(|e| err(500, &e.to_string()))?;
        use tokio::io::AsyncWriteExt;
        while let Some(chunk) = payload.next().await {
            let chunk = chunk.map_err(|e| err(400, &format!("upload failed: {e}")))?;
            size += chunk.len() as u64;
            if size > state.max_upload.load(std::sync::atomic::Ordering::Relaxed) {
                let _ = tokio::fs::remove_file(&tmp).await;
                return Err(err(413, "file exceeds the upload limit"));
            }
            f.write_all(&chunk).await.map_err(|e| err(500, &e.to_string()))?;
        }
        f.flush().await.map_err(|e| err(500, &e.to_string()))?;
    }
    if size == 0 {
        let _ = tokio::fs::remove_file(&tmp).await;
        return Err(err(400, "empty file"));
    }
    let dest = state.blob_path(&key);
    tokio::fs::rename(&tmp, &dest).await.map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        err(500, &e.to_string())
    })?;
    let mime = mime_guess::from_path(&name).first_or_octet_stream().essence_str().to_string();
    let node = store::create_node(
        &state.pool,
        parent,
        a.user.id,
        &name,
        &mime,
        size as i64,
        &key,
        "file",
    )
    .await?;
    let _ = state.audit(
        a.user.id,
        &a.user.name,
        "file.upload",
        "file",
        node.id,
        &format!("{} ({})", node.name, human_bytes(node.size)),
        &client_ip(&req),
    ).await;
    let users_map = users_map(&state.pool).await;
    let dto = file_dto(&node, &users_map, false, 0);
    j(&json!({"file": dto}))
}

fn content_disposition(name: &str, attachment: bool) -> String {
    let kind = if attachment { "attachment" } else { "inline" };
    let ascii: String = name
        .chars()
        .map(|c| if c.is_ascii() && (c.is_ascii_alphanumeric() || "._- ".contains(c)) { c } else { '_' })
        .collect();
    let encoded = utf8_percent_encode(name, NON_ALPHANUMERIC).to_string();
    format!("{kind}; filename=\"{ascii}\"; filename*=UTF-8''{encoded}")
}

pub async fn file_raw(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> ApiResult {
    let a = require(&req, &state).await?;
    let id = path.into_inner();
    let node = store::get_file(&state.pool, id).await?
        .ok_or_else(|| err(404, "file not found"))?;
    if node.kind != "file" {
        return Err(err(400, "not a file"));
    }
    let p = state.blob_path(&node.storage_key);
    let nf = NamedFile::open_async(&p).await.map_err(|_| err(404, "blob missing on disk"))?;
    let mut resp = nf.into_response(&req);
    resp.headers_mut()
        .insert(CONTENT_DISPOSITION, HeaderValue::from_str(&content_disposition(&node.name, false)).unwrap());
    Ok(resp)
}

pub async fn file_download(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> ApiResult {
    let a = require(&req, &state).await?;
    let id = path.into_inner();
    let node = store::get_file(&state.pool, id).await?
        .ok_or_else(|| err(404, "file not found"))?;
    if node.kind != "file" {
        return Err(err(400, "not a file"));
    }
    let p = state.blob_path(&node.storage_key);
    let nf = NamedFile::open_async(&p).await.map_err(|_| err(404, "blob missing on disk"))?;
    let mut resp = nf.into_response(&req);
    resp.headers_mut()
        .insert(CONTENT_DISPOSITION, HeaderValue::from_str(&content_disposition(&node.name, true)).unwrap());
    let _ = a;
    Ok(resp)
}

#[derive(Deserialize)]
pub struct NodeForm {
    pub parent: Option<i64>,
    pub name: Option<String>,
    pub kind: Option<String>,
}

pub async fn create_folder(
    req: HttpRequest,
    state: web::Data<AppState>,
    form: web::Json<NodeForm>,
) -> ApiResult {
    let a = require(&req, &state).await?;
    let f = form.into_inner();
    let parent = f.parent.unwrap_or(0);
    if parent > 0 {
        match store::get_file(&state.pool, parent).await? {
            Some(x) if x.kind == "folder" => {}
            _ => return Err(err(400, "parent folder not found")),
        }
    }
    let name = clean_node_name(f.name.as_deref().unwrap_or(""));
    if name == "Untitled" {
        return Err(err(400, "folder name is required"));
    }
    if store::file_by_name(&state.pool, parent, &name).await?.is_some() {
        return Err(err(409, "an item with that name already exists here"));
    }
    let node = store::create_node(
        &state.pool,
        parent,
        a.user.id,
        &name,
        "inode/directory",
        0,
        "",
        "folder",
    )
    .await?;
    let _ = state.audit(a.user.id, &a.user.name, "file.mkdir", "folder", node.id, &node.name, &client_ip(&req)).await;
    j(&json!({"ok": true, "id": node.id}))
}

#[derive(Deserialize)]
pub struct PatchNode {
    pub name: Option<String>,
    pub parent: Option<i64>,
}

pub async fn patch_file(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<i64>,
    form: web::Json<PatchNode>,
) -> ApiResult {
    let a = require(&req, &state).await?;
    let id = path.into_inner();
    let node = store::get_file(&state.pool, id).await?
        .ok_or_else(|| err(404, "not found"))?;
    let f = form.into_inner();
    let mut target_parent = node.parent_id;
    if let Some(p) = f.parent {
        if p != node.parent_id {
            if p == id {
                return Err(err(400, "cannot move a folder into itself"));
            }
            match store::get_file(&state.pool, p).await? {
                Some(x) if x.kind == "folder" => target_parent = p,
                _ => return Err(err(400, "target folder not found")),
            }
            // prevent moving into own descendant
            let mut cur = p;
            let mut guard = 0;
            while cur > 0 && guard < 64 {
                guard += 1;
                if cur == id {
                    return Err(err(400, "cannot move a folder into its own subfolder"));
                }
                match store::get_file(&state.pool, cur).await? {
                    Some(x) => cur = x.parent_id,
                    None => break,
                }
            }
        }
    }
    let name = match f.name {
        Some(n) if !n.trim().is_empty() => clean_node_name(&n),
        _ => node.name.clone(),
    };
    if name != node.name && store::file_by_name(&state.pool, target_parent, &name).await?.is_some() {
        return Err(err(409, "an item with that name already exists there"));
    }
    if name != node.name {
        store::rename_node(&state.pool, id, &name).await?;
    }
    if target_parent != node.parent_id {
        store::move_node(&state.pool, id, target_parent).await?;
    }
    let _ = state.audit(a.user.id, &a.user.name, "file.rename", "file", id, &name, &client_ip(&req)).await;
    j(&json!({"ok": true}))
}

#[derive(Deserialize)]
pub struct TrashForm {
    pub on: Option<bool>,
}

pub async fn trash_file(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<i64>,
    form: web::Json<TrashForm>,
) -> ApiResult {
    let a = require(&req, &state).await?;
    let id = path.into_inner();
    let node = store::get_file(&state.pool, id).await?
        .ok_or_else(|| err(404, "not found"))?;
    let on = form.into_inner().on.unwrap_or(true);
    store::trash_node(&state.pool, id, on).await?;
    let _ = state.audit(
        a.user.id,
        &a.user.name,
        if on { "file.delete" } else { "file.restore" },
        "file",
        id,
        &node.name,
        &client_ip(&req),
    ).await;
    j(&json!({"ok": true}))
}

pub async fn purge_file(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> ApiResult {
    let a = require(&req, &state).await?;
    let id = path.into_inner();
    let node = store::purge_node(&state.pool, id).await?;
    if node.kind == "file" && !node.storage_key.is_empty() {
        let _ = tokio::fs::remove_file(state.blob_path(&node.storage_key)).await;
    }
    let _ = state.audit(a.user.id, &a.user.name, "file.purge", "file", id, &node.name, &client_ip(&req)).await;
    j(&json!({"ok": true}))
}

pub async fn file_detail(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> ApiResult {
    let a = require(&req, &state).await?;
    let id = path.into_inner();
    let node = store::get_file(&state.pool, id).await?
        .ok_or_else(|| err(404, "not found"))?;
    let users = store::list_users(&state.pool).await.unwrap_or_default();
    let by_id: std::collections::HashMap<i64, User> = users.iter().cloned().map(|u| (u.id, u)).collect();
    let children = store::child_count(&state.pool, id).await?;
    let dto = file_dto(&node, &by_id, false, children);
    let _ = a;
    j(&json!({"file": dto}))
}

// ------------------------------------------------------------------ users / profile

pub async fn list_users(req: HttpRequest, state: web::Data<AppState>) -> ApiResult {
    let _a = require(&req, &state).await?;
    let users = store::list_users(&state.pool).await?;
    let dto: Vec<UserDto> = users.iter().map(UserDto::from).collect();
    j(&json!({"users": dto}))
}

#[derive(Deserialize)]
pub struct ProfileForm {
    pub name: String,
    pub bio: Option<String>,
}

pub async fn update_profile(
    req: HttpRequest,
    state: web::Data<AppState>,
    form: web::Json<ProfileForm>,
) -> ApiResult {
    let a = require(&req, &state).await?;
    let f = form.into_inner();
    let name = crate::util::clean_node_name(&f.name);
    if name == "Untitled" {
        return Err(err(400, "name is required"));
    }
    let bio = f.bio.unwrap_or_default().chars().take(240).collect();
    store::update_user(&state.pool, a.user.id, &name, &bio, &a.user.role, &a.user.state).await?;
    let u = store::get_user(&state.pool, a.user.id).await?;
    state.broadcast(json!({"t": "users:changed"}));
    j(&json!({"me": UserDto::from(&u)}))
}

// ------------------------------------------------------------------ admin

pub async fn admin_overview(req: HttpRequest, state: web::Data<AppState>) -> ApiResult {
    let _a = require_admin(&req, &state).await?;
    let stats = store::stats(&state.pool).await?;
    let audit = store::list_audit(&state.pool, 400).await?;
    let dto: Vec<AuditDto> = audit
        .iter()
        .map(|a| AuditDto {
            id: a.id,
            actor_name: a.actor_name.clone(),
            action: a.action.clone(),
            entity: a.entity.clone(),
            detail: a.detail.clone(),
            ip: a.ip.clone(),
            created_at: a.created_at,
        })
        .collect();
    j(&json!({"stats": store::json_map(&stats), "audit": dto}))
}

pub async fn admin_requests(req: HttpRequest, state: web::Data<AppState>) -> ApiResult {
    let _a = require_admin(&req, &state).await?;
    let devices = store::list_devices(&state.pool).await?;
    let users = store::list_users(&state.pool).await.unwrap_or_default();
    let by_id: std::collections::HashMap<i64, String> = users.into_iter().map(|u| (u.id, u.name)).collect();
    let pending: Vec<DeviceDto> = devices
        .iter()
        .filter(|d| d.approved == 0 && d.revoked == 0)
        .map(|d| DeviceDto {
            id: d.id,
            user_id: d.user_id,
            owner: by_id.get(&d.user_id).cloned().unwrap_or_default(),
            name: d.name.clone(),
            platform: d.platform.clone(),
            ip: d.ip.clone(),
            approved: false,
            revoked: false,
            current: false,
            requested_at: d.requested_at,
            approved_at: 0,
            last_seen_at: d.last_seen_at,
        })
        .collect();
    j(&json!({"pending": pending}))
}

pub async fn admin_approve(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> ApiResult {
    let a = require_admin(&req, &state).await?;
    let id = path.into_inner();
    store::approve_device(&state.pool, id).await?;
    let _ = state.audit(a.user.id, &a.user.name, "device.approve", "device", id, "approved", &client_ip(&req)).await;
    state.broadcast(json!({"t":"requests:changed"}));
    j(&json!({"ok": true}))
}

pub async fn admin_reject(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> ApiResult {
    let a = require_admin(&req, &state).await?;
    let id = path.into_inner();
    store::reject_device(&state.pool, id).await?;
    let _ = state.audit(a.user.id, &a.user.name, "device.reject", "device", id, "rejected", &client_ip(&req)).await;
    state.broadcast(json!({"t":"requests:changed"}));
    j(&json!({"ok": true}))
}

pub async fn admin_revoke(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> ApiResult {
    let a = require_admin(&req, &state).await?;
    let id = path.into_inner();
    if let Ok(d) = store::get_device(&state.pool, id).await {
        if d.user_id == a.user.id {
            return Err(err(400, "you cannot revoke your own device here"));
        }
    }
    store::revoke_device(&state.pool, id).await?;
    let _ = state.audit(a.user.id, &a.user.name, "device.revoke", "device", id, "revoked", &client_ip(&req)).await;
    state.broadcast(json!({"t":"devices:changed"}));
    j(&json!({"ok": true}))
}

#[derive(Deserialize)]
pub struct UserPatch {
    pub role: Option<String>,
    pub state: Option<String>,
}

pub async fn admin_update_user(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<i64>,
    form: web::Json<UserPatch>,
) -> ApiResult {
    let a = require_admin(&req, &state).await?;
    let id = path.into_inner();
    if id == a.user.id {
        return Err(err(400, "you cannot change your own role here"));
    }
    let target = store::get_user(&state.pool, id).await?;
    let f = form.into_inner();
    let role = match f.role {
        Some(r) if r == "owner" || r == "admin" || r == "member" => r,
        _ => target.role.clone(),
    };
    let st = match f.state {
        Some(s) if s == "active" || s == "suspended" => s,
        _ => target.state.clone(),
    };
    if role == "owner" && target.role != "owner" {
        return Err(err(400, "only one owner is supported"));
    }
    store::update_user(&state.pool, id, &target.name, &target.bio, &role, &st).await?;
    let _ = state.audit(a.user.id, &a.user.name, "user.update", "user", id, &format!("role={role} state={st}"), &client_ip(&req)).await;
    state.broadcast(json!({"t":"users:changed"}));
    j(&json!({"ok": true}))
}

pub async fn admin_audit(req: HttpRequest, state: web::Data<AppState>) -> ApiResult {
    let _a = require_admin(&req, &state).await?;
    let entries = store::list_audit(&state.pool, 500).await?;
    let dto: Vec<AuditDto> = entries
        .iter()
        .map(|a| AuditDto {
            id: a.id,
            actor_name: a.actor_name.clone(),
            action: a.action.clone(),
            entity: a.entity.clone(),
            detail: a.detail.clone(),
            ip: a.ip.clone(),
            created_at: a.created_at,
        })
        .collect();
    j(&json!({"audit": dto}))
}

pub async fn admin_backups(req: HttpRequest, state: web::Data<AppState>) -> ApiResult {
    let _a = require_admin(&req, &state).await?;
    let items = store::list_backups(&state.pool).await?;
    let dto: Vec<BackupDto> = items
        .iter()
        .map(|b| BackupDto { id: b.id, name: b.name.clone(), size: b.size, created_at: b.created_at })
        .collect();
    j(&json!({"backups": dto}))
}

pub async fn admin_create_backup(
    req: HttpRequest,
    state: web::Data<AppState>,
) -> ApiResult {
    let a = require_admin(&req, &state).await?;
    let dir = state.cfg.data_dir.join("backups");
    tokio::fs::create_dir_all(&dir).await.map_err(|e| err(500, &e.to_string()))?;
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let name = format!("room-{stamp}.db");
    let dest = dir.join(&name);
    // SQLite backup through SQLx: use the online backup API when possible,
    // otherwise fall back to a plain file copy while writes are quiescent.
    let escaped = dest.to_string_lossy().replace('\'', "''");
    let vacuum = format!("VACUUM INTO '{escaped}'");
    let copied = match sqlx::query(&vacuum).execute(&state.pool).await {
        Ok(_) => true,
        Err(_) => {
            let _ = sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)").execute(&state.pool).await;
            tokio::fs::copy(state.cfg.db_path(), &dest).await.is_ok()
        }
    };
    if !copied {
        return Err(err(500, "backup failed"));
    }
    let size = tokio::fs::metadata(&dest).await.map(|m| m.len() as i64).unwrap_or(0);
    let row = store::create_backup(&state.pool, &name, size, &dest.to_string_lossy()).await?;
    let _ = state.audit(a.user.id, &a.user.name, "backup.create", "backup", row.id, &name, &client_ip(&req)).await;
    j(&json!({"ok": true, "id": row.id, "size": size}))
}

pub async fn admin_delete_backup(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> ApiResult {
    let a = require_admin(&req, &state).await?;
    let id = path.into_inner();
    if let Ok(b) = sqlx::query_as::<_, (String,)>("SELECT path FROM backups WHERE id = ?")
        .bind(id)
        .fetch_one(&state.pool)
        .await
    {
        let _ = tokio::fs::remove_file(&b.0).await;
    }
    store::delete_backup(&state.pool, id).await?;
    let _ = state.audit(a.user.id, &a.user.name, "backup.delete", "backup", id, "", &client_ip(&req)).await;
    j(&json!({"ok": true}))
}

#[derive(Deserialize)]
pub struct ConfigForm {
    pub server_name: Option<String>,
    pub network_name: Option<String>,
    pub max_upload_mb: Option<String>,
    pub theme: Option<String>,
}

pub async fn admin_config(
    req: HttpRequest,
    state: web::Data<AppState>,
    form: web::Json<ConfigForm>,
) -> ApiResult {
    let a = require_admin(&req, &state).await?;
    let f = form.into_inner();
    if let Some(v) = f.server_name {
        let v = v.trim().to_string();
        if !v.is_empty() {
            store::set_setting(&state.pool, "server_name", &v).await?;
        }
    }
    if let Some(v) = f.network_name {
        store::set_setting(&state.pool, "network_name", v.trim()).await?;
    }
    if let Some(v) = f.max_upload_mb {
        if let Ok(n) = v.trim().parse::<u64>() {
            if n > 0 && n <= 102400 {
                state.max_upload.store(n * 1024 * 1024, std::sync::atomic::Ordering::Relaxed);
                store::set_setting(&state.pool, "max_upload_mb", &n.to_string()).await?;
            }
        }
    }
    if let Some(v) = f.theme {
        if ["light", "dark", "system"].contains(&v.as_str()) {
            store::set_setting(&state.pool, "theme", &v).await?;
        }
    }
    let _ = state.audit(a.user.id, &a.user.name, "config.update", "setting", 0, "configuration saved", &client_ip(&req)).await;
    j(&json!({"ok": true}))
}

pub async fn admin_shutdown(req: HttpRequest, state: web::Data<AppState>) -> ApiResult {
    let a = require_admin(&req, &state).await?;
    let _ = state.audit(a.user.id, &a.user.name, "server.shutdown", "server", 0, "shutdown requested", &client_ip(&req)).await;
    state.broadcast(json!({"t":"server:shutdown"}));
    actix_web::rt::spawn(async move {
        actix_web::rt::time::sleep(std::time::Duration::from_millis(400)).await;
        std::process::exit(0);
    });
    j(&json!({"ok": true}))
}

pub async fn admin_restart(req: HttpRequest, state: web::Data<AppState>) -> ApiResult {
    let a = require_admin(&req, &state).await?;
    let _ = state.audit(a.user.id, &a.user.name, "server.restart", "server", 0, "restart requested", &client_ip(&req)).await;
    state.broadcast(json!({"t":"server:restart"}));
    actix_web::rt::spawn(async move {
        actix_web::rt::time::sleep(std::time::Duration::from_millis(600)).await;
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            let exe = std::env::current_exe().unwrap_or_default();
            let args: Vec<String> = std::env::args().skip(1).collect();
            let _ = std::process::Command::new(exe).args(args).exec();
        }
        std::process::exit(0);
    });
    j(&json!({"ok": true}))
}

// ------------------------------------------------------------------ calls

pub fn call_dto(call: &Call, users: &[UserDto], participants: &[(i64, i64)]) -> CallDto {
    let active: Vec<i64> = participants
        .iter()
        .filter(|(_, left)| *left == 0)
        .map(|(uid, _)| *uid)
        .collect();
    let names: Vec<String> = active
        .iter()
        .filter_map(|id| users.iter().find(|u| u.id == *id).map(|u| u.name.clone()))
        .collect();
    CallDto {
        id: call.id,
        kind: call.kind.clone(),
        status: call.status.clone(),
        started_by: call.started_by.unwrap_or(0),
        starter_name: call
            .started_by
            .and_then(|id| users.iter().find(|u| u.id == id))
            .map(|u| u.name.clone())
            .unwrap_or_default(),
        conversation_id: call.conversation_id.unwrap_or(0),
        participants: active,
        participant_names: names,
        started_at: call.started_at,
        ended_at: call.ended_at,
        duration_ms: if call.ended_at > 0 { call.ended_at - call.started_at } else { 0 },
    }
}

pub async fn calls_index(req: HttpRequest, state: web::Data<AppState>) -> ApiResult {
    let a = require(&req, &state).await?;
    let users = store::list_users(&state.pool).await.unwrap_or_default();
    let user_dtos: Vec<UserDto> = users.iter().map(UserDto::from).collect();
    let history = store::call_history(&state.pool, 30).await?;
    let mut dtos = Vec::new();
    for c in &history {
        let parts = store::call_participants(&state.pool, c.id).await.unwrap_or_default();
        dtos.push(call_dto(c, &user_dtos, &parts));
    }
    let active = match store::active_call(&state.pool).await? {
        Some(c) => {
            let parts = store::call_participants(&state.pool, c.id).await.unwrap_or_default();
            Some(call_dto(&c, &user_dtos, &parts))
        }
        None => None,
    };
    let convs = store::list_conversations(&state.pool, a.user.id).await?;
    j(&json!({"calls": dtos, "active": active, "conversations": convs}))
}

#[derive(Deserialize)]
pub struct StartCall {
    pub conversation_id: i64,
    pub kind: Option<String>,
}

pub async fn calls_start(
    req: HttpRequest,
    state: web::Data<AppState>,
    form: web::Json<StartCall>,
) -> ApiResult {
    let a = require(&req, &state).await?;
    let f = form.into_inner();
    let conv = store::get_conversation(&state.pool, f.conversation_id).await?
        .ok_or_else(|| err(404, "conversation not found"))?;
    if !store::is_member(&state.pool, conv.id, a.user.id).await? {
        return Err(err(403, "not a member"));
    }
    if let Some(active) = store::active_call(&state.pool).await? {
        if active.status == "active" {
            let parts = store::call_participants(&state.pool, active.id).await.unwrap_or_default();
            let users = store::list_users(&state.pool).await.unwrap_or_default();
            let user_dtos: Vec<UserDto> = users.iter().map(UserDto::from).collect();
            return j(&json!({"active": call_dto(&active, &user_dtos, &parts), "already": true}));
        }
    }
    let kind = if f.kind.as_deref() == Some("audio") { "audio" } else { "video" };
    let call = store::create_call(&state.pool, kind, conv.id, a.user.id).await?;
    let members: Vec<i64> = store::conversation_members(&state.pool, conv.id)
        .await?
        .into_iter()
        .map(|m| m.user_id)
        .filter(|id| *id != a.user.id)
        .collect();
    state.to_users(
        members,
        json!({"t":"call:incoming","call_id":call.id,"kind":kind,"conversation_id":conv.id,"from":a.user.name}),
    );
    let _ = state.audit(a.user.id, &a.user.name, "call.start", "call", call.id, kind, &client_ip(&req)).await;
    j(&json!({"call_id": call.id, "kind": kind}))
}

pub async fn calls_join(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> ApiResult {
    let a = require(&req, &state).await?;
    let id = path.into_inner();
    let call = store::get_call(&state.pool, id).await?
        .ok_or_else(|| err(404, "call not found"))?;
    store::join_call(&state.pool, id, a.user.id).await?;
    let parts = store::call_participants(&state.pool, id).await.unwrap_or_default();
    let users = store::list_users(&state.pool).await.unwrap_or_default();
    let user_dtos: Vec<UserDto> = users.iter().map(UserDto::from).collect();
    let dto = call_dto(&call, &user_dtos, &parts);
    let members: Vec<i64> = parts.iter().map(|(uid, _)| *uid).filter(|u| *u != a.user.id).collect();
    state.to_users(members, json!({"t":"call:peer","call_id":id,"user_id":a.user.id,"event":"join"}));
    j(&json!({"call": dto}))
}

pub async fn calls_leave(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> ApiResult {
    let a = require(&req, &state).await?;
    let id = path.into_inner();
    store::leave_call(&state.pool, id, a.user.id).await?;
    let parts = store::call_participants(&state.pool, id).await.unwrap_or_default();
    let members: Vec<i64> = parts.iter().map(|(uid, _)| *uid).filter(|u| *u != a.user.id).collect();
    state.to_users(members, json!({"t":"call:peer","call_id":id,"user_id":a.user.id,"event":"leave"}));
    let remaining = parts.iter().filter(|(_, left)| *left == 0).count();
    if remaining == 0 {
        store::end_call(&state.pool, id).await?;
        state.broadcast(json!({"t":"call:ended","call_id":id}));
    }
    j(&json!({"ok": true}))
}

pub async fn calls_end(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> ApiResult {
    let a = require(&req, &state).await?;
    let id = path.into_inner();
    store::end_call(&state.pool, id).await?;
    state.broadcast(json!({"t":"call:ended","call_id":id,"by":a.user.id}));
    j(&json!({"ok": true}))
}

// ------------------------------------------------------------------ websocket

pub async fn ws_event(
    state: &web::Data<AppState>,
    _hub: &Addr<Hub>,
    uid: i64,
    msg: Value,
) -> Result<Option<Value>, String> {
    if uid == 0 {
        return Err("not authorised".into());
    }
    let t = msg.get("t").and_then(|v| v.as_str()).unwrap_or("");
    match t {
        "ping" => Ok(Some(json!({"t":"pong","ts": now_ms()}))),
        "presence:active" => {
            let _ = store::set_presence(&state.pool, uid, "online").await;
            let _ = store::touch_user(&state.pool, uid).await;
            Ok(None)
        }
        "presence:idle" => {
            let _ = store::set_presence(&state.pool, uid, "away").await;
            Ok(None)
        }
        "chat:send" => {
            let cid = msg.get("conversation_id").and_then(|v| v.as_i64()).unwrap_or(0);
            let body = msg.get("body").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let reply_to_id = msg.get("reply_to_id").and_then(|v| v.as_i64()).unwrap_or(0);
            if cid == 0 || body.trim().is_empty() {
                return Err("invalid message".into());
            }
            if body.chars().count() > 8000 {
                return Err("message too long".into());
            }
            if !store::is_member(&state.pool, cid, uid).await.map_err(|e| e.to_string())? {
                return Err("not a member".into());
            }
            let m = store::insert_message(
                &state.pool,
                &store::NewMessage { conversation_id: cid, sender_id: uid, body, reply_to_id },
            )
            .await
            .map_err(|e| e.to_string())?;
            let dto = messages_dto(&state.pool, std::slice::from_ref(&m), uid)
                .await
                .map_err(|e| e.to_string())?;
            let dto = dto.into_iter().next().unwrap();
            let members: Vec<i64> = store::conversation_members(&state.pool, cid)
                .await
                .map_err(|e| e.to_string())?
                .into_iter()
                .map(|m| m.user_id)
                .collect();
            broadcast_msg(state, &dto, &members);
            Ok(Some(json!({"t":"message:ack","message":dto})))
        }
        "chat:typing" => {
            let cid = msg.get("conversation_id").and_then(|v| v.as_i64()).unwrap_or(0);
            if cid > 0 {
                if let Ok(members) = store::conversation_members(&state.pool, cid).await {
                    let list: Vec<i64> = members.into_iter().map(|m| m.user_id).filter(|u| *u != uid).collect();
                    let name = store::get_user(&state.pool, uid).await.map(|u| u.name).unwrap_or_default();
                    state.to_users(list, json!({"t":"typing","conversation_id":cid,"user_id":uid,"name":name}));
                }
            }
            Ok(None)
        }
        "chat:read" => {
            let cid = msg.get("conversation_id").and_then(|v| v.as_i64()).unwrap_or(0);
            if cid > 0 {
                let _ = store::mark_read(&state.pool, cid, uid).await;
                let _ = store::touch_user(&state.pool, uid).await;
            }
            Ok(None)
        }
        "react" => {
            let mid = msg.get("message_id").and_then(|v| v.as_i64()).unwrap_or(0);
            let emoji = msg.get("emoji").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
            let on = msg.get("on").and_then(|v| v.as_bool()).unwrap_or(true);
            if mid == 0 || emoji.is_empty() || emoji.chars().count() > 8 {
                return Err("invalid reaction".into());
            }
            let m = store::get_message(&state.pool, mid).await.map_err(|e| e.to_string())?
                .ok_or_else(|| "message not found".to_string())?;
            store::set_reaction(&state.pool, mid, uid, &emoji, on).await.map_err(|e| e.to_string())?;
            let fresh = store::get_message(&state.pool, mid).await.map_err(|e| e.to_string())?.unwrap();
            let dto = messages_dto(&state.pool, std::slice::from_ref(&fresh), uid).await.map_err(|e| e.to_string())?;
            let dto = dto.into_iter().next().unwrap();
            let members: Vec<i64> = store::conversation_members(&state.pool, m.conversation_id)
                .await.map_err(|e| e.to_string())?.into_iter().map(|m| m.user_id).collect();
            state.to_users(members, json!({"t":"message:updated","message":dto}));
            Ok(Some(json!({"t":"message:updated","message":dto})))
        }
        "call:start" => {
            let cid = msg.get("conversation_id").and_then(|v| v.as_i64()).unwrap_or(0);
            let kind = msg.get("kind").and_then(|v| v.as_str()).unwrap_or("video").to_string();
            if cid == 0 {
                return Err("conversation required".into());
            }
            if let Some(active) = store::active_call(&state.pool).await.map_err(|e| e.to_string())? {
                return Ok(Some(json!({"t":"call:active","call_id":active.id})));
            }
            let call = store::create_call(&state.pool, &kind, cid, uid).await.map_err(|e| e.to_string())?;
            let members: Vec<i64> = store::conversation_members(&state.pool, cid)
                .await.map_err(|e| e.to_string())?.into_iter().map(|m| m.user_id).filter(|u| *u != uid).collect();
            let name = store::get_user(&state.pool, uid).await.map(|u| u.name).unwrap_or_default();
            state.to_users(members, json!({"t":"call:incoming","call_id":call.id,"kind":kind,"conversation_id":cid,"from":name}));
            Ok(Some(json!({"t":"call:started","call_id":call.id})))
        }
        "call:join" => {
            let id = msg.get("call_id").and_then(|v| v.as_i64()).unwrap_or(0);
            if id == 0 { return Err("call required".into()); }
            store::join_call(&state.pool, id, uid).await.map_err(|e| e.to_string())?;
            let parts = store::call_participants(&state.pool, id).await.unwrap_or_default();
            let members: Vec<i64> = parts.iter().map(|(u, _)| *u).filter(|u| *u != uid).collect();
            state.to_users(members, json!({"t":"call:peer","call_id":id,"user_id":uid,"event":"join"}));
            Ok(Some(json!({"t":"call:joined","call_id":id})))
        }
        "call:leave" => {
            let id = msg.get("call_id").and_then(|v| v.as_i64()).unwrap_or(0);
            if id > 0 {
                store::leave_call(&state.pool, id, uid).await.map_err(|e| e.to_string())?;
                let parts = store::call_participants(&state.pool, id).await.unwrap_or_default();
                let members: Vec<i64> = parts.iter().map(|(u, _)| *u).filter(|u| *u != uid).collect();
                state.to_users(members, json!({"t":"call:peer","call_id":id,"user_id":uid,"event":"leave"}));
                let remaining = parts.iter().filter(|(_, left)| *left == 0).count();
                if remaining == 0 {
                    store::end_call(&state.pool, id).await.map_err(|e| e.to_string())?;
                    state.broadcast(json!({"t":"call:ended","call_id":id}));
                }
            }
            Ok(None)
        }
        "call:end" => {
            let id = msg.get("call_id").and_then(|v| v.as_i64()).unwrap_or(0);
            if id > 0 {
                store::end_call(&state.pool, id).await.map_err(|e| e.to_string())?;
                state.broadcast(json!({"t":"call:ended","call_id":id}));
            }
            Ok(None)
        }
        "call:signal" => {
            let id = msg.get("call_id").and_then(|v| v.as_i64()).unwrap_or(0);
            let to = msg.get("to").and_then(|v| v.as_i64()).unwrap_or(0);
            let data = msg.get("data").cloned().unwrap_or(Value::Null);
            if id == 0 || to == 0 {
                return Err("signal target missing".into());
            }
            let from = store::get_user(&state.pool, uid).await.map(|u| u.name).unwrap_or_default();
            state.to_user(to, json!({"t":"call:signal","call_id":id,"from":uid,"from_name":from,"data":data}));
            Ok(None)
        }
        _ => Err(format!("unknown event: {t}")),
    }
}

pub fn server_info(state: &AppState) -> Value {
    json!({
        "name": state.cfg.server_name,
        "version": state.version,
        "host": state.hostname,
        "uptime_ms": state.uptime_ms(),
        "network": state.cfg.lan_ssid_hint,
    })
}

pub fn gate_redirect() -> HttpResponse {
    HttpResponse::Found().append_header(("location", "/gate")).finish()
}
