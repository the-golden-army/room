use serde_json::Value;
use sqlx::{FromRow, SqlitePool};

use crate::models::*;
use crate::util::*;

// ------------------------------------------------------------------ users

pub async fn get_user(pool: &SqlitePool, id: i64) -> Result<User, sqlx::Error> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await
}

pub async fn user_by_handle(pool: &SqlitePool, handle: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE handle = ? COLLATE NOCASE")
        .bind(handle)
        .fetch_optional(pool)
        .await
}

pub async fn list_users(pool: &SqlitePool) -> Result<Vec<User>, sqlx::Error> {
    sqlx::query_as::<_, User>("SELECT * FROM users ORDER BY name COLLATE NOCASE")
        .fetch_all(pool)
        .await
}

pub async fn create_user(
    pool: &SqlitePool,
    name: &str,
    handle: &str,
    role: &str,
) -> Result<User, sqlx::Error> {
    let now = now_ms();
    let id = sqlx::query("INSERT INTO users (name, handle, role, created_at, last_seen_at) VALUES (?,?,?,?,?)")
        .bind(name)
        .bind(handle)
        .bind(role)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await?
        .last_insert_rowid();
    get_user(pool, id).await
}

pub async fn update_user(
    pool: &SqlitePool,
    id: i64,
    name: &str,
    bio: &str,
    role: &str,
    state: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET name = ?, bio = ?, role = ?, state = ? WHERE id = ?")
        .bind(name)
        .bind(bio)
        .bind(role)
        .bind(state)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_presence(pool: &SqlitePool, id: i64, p: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET presence = ? WHERE id = ?")
        .bind(p)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn touch_user(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
    let now = now_ms();
    sqlx::query("UPDATE users SET last_seen_at = ? WHERE id = ?")
        .bind(now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub struct PresenceSnapshot {
    pub online: Vec<i64>,
}

pub async fn presence_snapshot(pool: &SqlitePool) -> PresenceSnapshot {
    let online = sqlx::query_as::<_, (i64,)>(
        "SELECT id FROM users WHERE presence = 'online' AND state = 'active'",
    )
    .fetch_all(pool)
    .await
    .map(|rows| rows.into_iter().map(|r| r.0).collect())
    .unwrap_or_default();
    PresenceSnapshot { online }
}

pub async fn reaper(pool: &SqlitePool) {
    let cutoff = now_ms() - 90_000;
    let _ = sqlx::query("UPDATE users SET presence = 'offline' WHERE presence = 'online' AND last_seen_at < ?")
        .bind(cutoff)
        .execute(pool)
        .await;
}

// ------------------------------------------------------------------ devices

pub async fn device_by_hash(
    pool: &SqlitePool,
    hash: &str,
) -> Result<Option<Device>, sqlx::Error> {
    sqlx::query_as::<_, Device>("SELECT * FROM devices WHERE token_hash = ?")
        .bind(hash)
        .fetch_optional(pool)
        .await
}

pub async fn get_device(pool: &SqlitePool, id: i64) -> Result<Device, sqlx::Error> {
    sqlx::query_as::<_, Device>("SELECT * FROM devices WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await
}

pub async fn list_devices(pool: &SqlitePool) -> Result<Vec<Device>, sqlx::Error> {
    sqlx::query_as::<_, Device>("SELECT * FROM devices ORDER BY requested_at DESC")
        .fetch_all(pool)
        .await
}

pub async fn devices_for_user(pool: &SqlitePool, uid: i64) -> Result<Vec<Device>, sqlx::Error> {
    sqlx::query_as::<_, Device>("SELECT * FROM devices WHERE user_id = ? ORDER BY requested_at DESC")
        .bind(uid)
        .fetch_all(pool)
        .await
}

#[allow(clippy::too_many_arguments)]
pub async fn create_device(
    pool: &SqlitePool,
    user_id: i64,
    name: &str,
    platform: &str,
    token_hash: &str,
    fingerprint: &str,
    ip: &str,
    approved: bool,
) -> Result<Device, sqlx::Error> {
    let now = now_ms();
    let id = sqlx::query(
        "INSERT INTO devices (user_id, name, platform, token_hash, fingerprint, ip, approved, requested_at, approved_at, last_seen_at)
         VALUES (?,?,?,?,?,?,?,?,?,?)",
    )
    .bind(user_id)
    .bind(name)
    .bind(platform)
    .bind(token_hash)
    .bind(fingerprint)
    .bind(ip)
    .bind(if approved { 1 } else { 0 })
    .bind(now)
    .bind(if approved { now } else { 0 })
    .bind(now)
    .execute(pool)
    .await?
    .last_insert_rowid();
    get_device(pool, id).await
}

pub async fn approve_device(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
    let now = now_ms();
    sqlx::query("UPDATE devices SET approved = 1, revoked = 0, approved_at = ? WHERE id = ?")
        .bind(now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn reject_device(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM devices WHERE id = ? AND approved = 0")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn revoke_device(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE devices SET revoked = 1, approved = 0 WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn touch_device(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
    let now = now_ms();
    sqlx::query("UPDATE devices SET last_seen_at = ? WHERE id = ?")
        .bind(now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn count_pending(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    let r = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM devices WHERE approved = 0 AND revoked = 0",
    )
    .fetch_one(pool)
    .await?;
    Ok(r.0)
}

pub async fn owner_count(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    let r = sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM users WHERE role = 'owner'")
        .fetch_one(pool)
        .await?;
    Ok(r.0)
}

// ------------------------------------------------------------------ audit

#[allow(clippy::too_many_arguments)]
pub async fn audit(
    pool: &SqlitePool,
    actor_id: i64,
    actor_name: &str,
    action: &str,
    entity: &str,
    entity_id: i64,
    detail: &str,
    ip: &str,
) {
    let _ = sqlx::query(
        "INSERT INTO audit_log (actor_id, actor_name, action, entity, entity_id, detail, ip, created_at)
         VALUES (?,?,?,?,?,?,?,?)",
    )
    .bind(actor_id)
    .bind(actor_name)
    .bind(action)
    .bind(entity)
    .bind(entity_id)
    .bind(detail)
    .bind(ip)
    .bind(now_ms())
    .execute(pool)
    .await;
}

pub async fn list_audit(pool: &SqlitePool, limit: i64) -> Result<Vec<AuditEntry>, sqlx::Error> {
    sqlx::query_as::<_, AuditEntry>("SELECT * FROM audit_log ORDER BY id DESC LIMIT ?")
        .bind(limit)
        .fetch_all(pool)
        .await
}

// ------------------------------------------------------------------ settings

pub async fn all_settings(pool: &SqlitePool) -> Result<std::collections::BTreeMap<String, String>, sqlx::Error> {
    let rows: Vec<SettingRow> = sqlx::query_as("SELECT * FROM settings").fetch_all(pool).await?;
    Ok(rows.into_iter().map(|r| (r.key, r.value)).collect())
}

// ------------------------------------------------------------------ conversations

#[derive(FromRow)]
struct ConvRow {
    id: i64,
    kind: String,
    name: String,
    topic: String,
    color: String,
    created_by: Option<i64>,
    created_at: i64,
    last_message_at: i64,
    last_preview: String,
    last_sender: String,
    last_read_id: i64,
    muted: i32,
    unread: i64,
}

pub async fn list_conversations(
    pool: &SqlitePool,
    uid: i64,
) -> Result<Vec<ConvDto>, sqlx::Error> {
    let rows: Vec<ConvRow> = sqlx::query_as(
        "SELECT c.*, cm.last_read_id, cm.muted,
           (SELECT COUNT(*) FROM messages m WHERE m.conversation_id = c.id AND m.id > cm.last_read_id AND m.deleted_at = 0) AS unread
         FROM conversations c
         JOIN conversation_members cm ON cm.conversation_id = c.id
         WHERE cm.user_id = ?
         ORDER BY c.last_message_at DESC, c.id DESC",
    )
    .bind(uid)
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Ok(vec![]);
    }
    let ids: Vec<i64> = rows.iter().map(|r| r.id).collect();
    let mut members: std::collections::HashMap<i64, Vec<i64>> = std::collections::HashMap::new();
    let placeholders = vec!["?"; ids.len()].join(",");
    let q = format!(
        "SELECT conversation_id, user_id FROM conversation_members WHERE conversation_id IN ({})",
        placeholders
    );
    let mut query = sqlx::query_as::<_, (i64, i64)>(&q);
    for id in &ids {
        query = query.bind(*id);
    }
    let mrows: Vec<(i64, i64)> = query.fetch_all(pool).await?;
    for (cid, uid) in mrows {
        members.entry(cid).or_default().push(uid);
    }

    let users = list_users(pool).await.unwrap_or_default();
    let by_id: std::collections::HashMap<i64, User> =
        users.into_iter().map(|u| (u.id, u)).collect();

    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        let mem = members.get(&r.id).cloned().unwrap_or_default();
        let (title, color, initials) = if r.kind == "group" {
            let c = if r.color.is_empty() { avatar_color(&format!("conv{}", r.id)) } else { r.color.clone() };
            (r.name.clone(), c.clone(), initials(&r.name))
        } else {
            let other = mem.iter().find(|x| **x != uid).and_then(|x| by_id.get(x));
            match other {
                Some(u) => (u.name.clone(), u.color.clone(), initials(&u.name)),
                None => ("Direct".into(), avatar_color(&format!("conv{}", r.id)), "?".into()),
            }
        };
        out.push(ConvDto {
            id: r.id,
            kind: r.kind,
            name: r.name,
            title,
            topic: r.topic,
            color,
            initials,
            members: mem,
            unread: r.unread,
            muted: r.muted == 1,
            last_message_at: r.last_message_at,
            last_preview: r.last_preview,
            last_sender: r.last_sender,
            my_last_read: r.last_read_id,
        });
    }
    Ok(out)
}

pub async fn get_conversation(
    pool: &SqlitePool,
    id: i64,
) -> Result<Option<Conversation>, sqlx::Error> {
    sqlx::query_as::<_, Conversation>("SELECT * FROM conversations WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn conversation_members(
    pool: &SqlitePool,
    cid: i64,
) -> Result<Vec<MemberRow>, sqlx::Error> {
    sqlx::query_as::<_, MemberRow>(
        "SELECT * FROM conversation_members WHERE conversation_id = ? ORDER BY joined_at",
    )
    .bind(cid)
    .fetch_all(pool)
    .await
}

pub async fn is_member(pool: &SqlitePool, cid: i64, uid: i64) -> Result<bool, sqlx::Error> {
    let r = sqlx::query_as::<_, (i64,)>(
        "SELECT 1 FROM conversation_members WHERE conversation_id = ? AND user_id = ?",
    )
    .bind(cid)
    .bind(uid)
    .fetch_optional(pool)
    .await?;
    Ok(r.is_some())
}

pub async fn create_direct(
    pool: &SqlitePool,
    a: i64,
    b: i64,
) -> Result<Conversation, sqlx::Error> {
    let exists: Option<(i64,)> = sqlx::query_as(
        "SELECT c.id FROM conversations c
         JOIN conversation_members m1 ON m1.conversation_id = c.id AND m1.user_id = ?
         JOIN conversation_members m2 ON m2.conversation_id = c.id AND m2.user_id = ?
         WHERE c.kind = 'direct'",
    )
    .bind(a)
    .bind(b)
    .fetch_optional(pool)
    .await?;
    if let Some((id,)) = exists {
        return sqlx::query_as::<_, Conversation>("SELECT * FROM conversations WHERE id = ?")
            .bind(id)
            .fetch_one(pool)
            .await;
    }
    let now = now_ms();
    let id = sqlx::query(
        "INSERT INTO conversations (kind, name, created_by, created_at) VALUES ('direct','',?,?,?)",
    )
    .bind(a)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?
    .last_insert_rowid();
    for u in [a, b] {
        sqlx::query(
            "INSERT OR IGNORE INTO conversation_members (conversation_id, user_id, role, joined_at) VALUES (?,?,'member',?)",
        )
        .bind(id)
        .bind(u)
        .bind(now)
        .execute(pool)
        .await?;
    }
    sqlx::query_as::<_, Conversation>("SELECT * FROM conversations WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await
}

pub async fn create_group(
    pool: &SqlitePool,
    name: &str,
    topic: &str,
    created_by: i64,
    members: &[i64],
) -> Result<Conversation, sqlx::Error> {
    let now = now_ms();
    let id = sqlx::query(
        "INSERT INTO conversations (kind, name, topic, color, created_by, created_at) VALUES ('group',?,?,?,?,?)",
    )
    .bind(name)
    .bind(topic)
    .bind(avatar_color(name))
    .bind(created_by)
    .bind(now)
    .execute(pool)
    .await?
    .last_insert_rowid();
    let mut all: Vec<i64> = members.to_vec();
    if !all.contains(&created_by) {
        all.push(created_by);
    }
    for u in all {
        sqlx::query(
            "INSERT OR IGNORE INTO conversation_members (conversation_id, user_id, role, joined_at) VALUES (?,?,'member',?)",
        )
        .bind(id)
        .bind(u)
        .bind(now)
        .execute(pool)
        .await?;
    }
    sqlx::query_as::<_, Conversation>("SELECT * FROM conversations WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await
}

pub async fn mark_read(pool: &SqlitePool, cid: i64, uid: i64) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE conversation_members SET last_read_id = (SELECT COALESCE(MAX(id),0) FROM messages WHERE conversation_id = ?)
         WHERE conversation_id = ? AND user_id = ?",
    )
    .bind(cid)
    .bind(cid)
    .bind(uid)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_muted(
    pool: &SqlitePool,
    cid: i64,
    uid: i64,
    muted: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE conversation_members SET muted = ? WHERE conversation_id = ? AND user_id = ?")
        .bind(if muted { 1 } else { 0 })
        .bind(cid)
        .bind(uid)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_conversation(
    pool: &SqlitePool,
    cid: i64,
    name: &str,
    topic: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE conversations SET name = ?, topic = ? WHERE id = ?")
        .bind(name)
        .bind(topic)
        .bind(cid)
        .execute(pool)
        .await?;
    Ok(())
}

// ------------------------------------------------------------------ messages

#[derive(FromRow)]
struct MsgRow {
    id: i64,
    conversation_id: i64,
    sender_id: Option<i64>,
    body: String,
    reply_to_id: i64,
    edited_at: i64,
    deleted_at: i64,
    created_at: i64,
    sender_name: Option<String>,
    sender_avatar: Option<String>,
    sender_color: Option<String>,
}

pub async fn get_message(pool: &SqlitePool, id: i64) -> Result<Option<Message>, sqlx::Error> {
    sqlx::query_as::<_, Message>("SELECT * FROM messages WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// Fetch a page of messages. `before` = message id to page backwards from (0 = latest).
pub async fn list_messages(
    pool: &SqlitePool,
    cid: i64,
    before: i64,
    limit: i64,
) -> Result<Vec<Message>, sqlx::Error> {
    let rows: Vec<Message> = if before > 0 {
        sqlx::query_as::<_, Message>(
            "SELECT * FROM messages WHERE conversation_id = ? AND id < ? AND deleted_at = 0
             ORDER BY id DESC LIMIT ?",
        )
        .bind(cid)
        .bind(before)
        .bind(limit)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, Message>(
            "SELECT * FROM messages WHERE conversation_id = ? AND deleted_at = 0
             ORDER BY id DESC LIMIT ?",
        )
        .bind(cid)
        .bind(limit)
        .fetch_all(pool)
        .await?
    };
    let mut v = rows;
    v.reverse();
    Ok(v)
}

pub async fn message_count(pool: &SqlitePool, _cid: i64) -> Result<i64, sqlx::Error> {
    let r = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM messages WHERE conversation_id = ? AND deleted_at = 0",
    )
    .fetch_one(pool)
    .await?;
    Ok(r.0)
}

pub struct NewMessage {
    pub conversation_id: i64,
    pub sender_id: i64,
    pub body: String,
    pub reply_to_id: i64,
}

pub async fn insert_message(
    pool: &SqlitePool,
    m: &NewMessage,
) -> Result<Message, sqlx::Error> {
    let now = now_ms();
    let id = sqlx::query(
        "INSERT INTO messages (conversation_id, sender_id, body, reply_to_id, created_at) VALUES (?,?,?,?,?)",
    )
    .bind(m.conversation_id)
    .bind(m.sender_id)
    .bind(&m.body)
    .bind(m.reply_to_id)
    .bind(now)
    .execute(pool)
    .await?
    .last_insert_rowid();
    let preview = truncate(&m.body.replace(['\n', '\r'], " "), 140);
    sqlx::query(
        "UPDATE conversations SET last_message_at = ?, last_preview = ?,
           last_sender = (SELECT name FROM users WHERE id = ?) WHERE id = ?",
    )
    .bind(now)
    .bind(&preview)
    .bind(m.sender_id)
    .bind(m.conversation_id)
    .execute(pool)
    .await?;
    sqlx::query("UPDATE conversation_members SET last_read_id = ? WHERE conversation_id = ? AND user_id = ?")
        .bind(id)
        .bind(m.conversation_id)
        .bind(m.sender_id)
        .execute(pool)
        .await?;
    get_message(pool, id).await.map(|o| o.unwrap())
}

pub async fn edit_message(
    pool: &SqlitePool,
    id: i64,
    body: &str,
) -> Result<Option<Message>, sqlx::Error> {
    let now = now_ms();
    let n = sqlx::query("UPDATE messages SET body = ?, edited_at = ? WHERE id = ? AND deleted_at = 0")
        .bind(body)
        .bind(now)
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected();
    if n == 0 {
        return Ok(None);
    }
    get_message(pool, id).await
}

pub async fn delete_message(pool: &SqlitePool, id: i64) -> Result<bool, sqlx::Error> {
    let n = sqlx::query("UPDATE messages SET deleted_at = ?, body = '' WHERE id = ? AND deleted_at = 0")
        .bind(now_ms())
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected();
    Ok(n > 0)
}

pub async fn reactions_for(
    pool: &SqlitePool,
    ids: &[i64],
) -> Result<Vec<ReactionRow>, sqlx::Error> {
    if ids.is_empty() {
        return Ok(vec![]);
    }
    let placeholders = vec!["?"; ids.len()].join(",");
    let q = format!(
        "SELECT message_id, user_id, emoji FROM reactions WHERE message_id IN ({})",
        placeholders
    );
    let mut query = sqlx::query_as::<_, ReactionRow>(&q);
    for id in ids {
        query = query.bind(*id);
    }
    query.fetch_all(pool).await
}

pub async fn set_reaction(
    pool: &SqlitePool,
    mid: i64,
    uid: i64,
    emoji: &str,
    on: bool,
) -> Result<(), sqlx::Error> {
    if on {
        sqlx::query(
            "INSERT OR IGNORE INTO reactions (message_id, user_id, emoji) VALUES (?,?,?)",
        )
        .bind(mid)
        .bind(uid)
        .bind(emoji)
        .execute(pool)
        .await?;
    } else {
        sqlx::query("DELETE FROM reactions WHERE message_id = ? AND user_id = ? AND emoji = ?")
            .bind(mid)
            .bind(uid)
            .bind(emoji)
            .execute(pool)
            .await?;
    }
    Ok(())
}

/// Build the wire representation of a message.
pub fn message_dto(
    m: &Message,
    users: &std::collections::HashMap<i64, User>,
    reactions: Vec<ReactionRow>,
    me: i64,
    replies: &std::collections::HashMap<i64, (String, String)>,
) -> MsgDto {
    let sender = m.sender_id.and_then(|id| users.get(&id));
    let sname = sender.map(|u| u.name.clone()).unwrap_or_else(|| "Unknown".into());
    let savatar = sender.map(|u| u.avatar.clone()).unwrap_or_default();
    let scolor = sender
        .map(|u| if u.color.is_empty() { avatar_color(&u.handle) } else { u.color.clone() })
        .unwrap_or_else(|| "#6b7280".into());
    let (reply_name, reply_body) = if m.reply_to_id > 0 {
        replies
            .get(&m.reply_to_id)
            .cloned()
            .unwrap_or_else(|| ("".into(), "".into()))
    } else {
        ("".into(), "".into())
    };
    let mut groups: Vec<ReactionDto> = vec![];
    for r in reactions {
        if let Some(g) = groups.iter_mut().find(|g| g.emoji == r.emoji) {
            g.users.push(r.user_id);
            if let Some(u) = users.get(&r.user_id) {
                g.names.push(u.name.clone());
            }
        } else {
            groups.push(ReactionDto {
                emoji: r.emoji,
                users: vec![r.user_id],
                names: users
                    .get(&r.user_id)
                    .map(|u| vec![u.name.clone()])
                    .unwrap_or_default(),
                mine: r.user_id == me,
            });
        }
    }
    for g in groups.iter_mut() {
        g.mine = g.users.contains(&me);
    }
    MsgDto {
        id: m.id,
        conversation_id: m.conversation_id,
        sender_id: m.sender_id.unwrap_or(0),
        sender_name: sname,
        sender_avatar: savatar,
        sender_color: scolor,
        sender_initials: initials(
            &m.sender_id
                .and_then(|id| users.get(&id))
                .map(|u| u.name.clone())
                .unwrap_or_default(),
        ),
        body: m.body.clone(),
        clean: m.body.clone(),
        reply_to_id: m.reply_to_id,
        reply_to_name: reply_name,
        reply_to_body: reply_body,
        reactions: groups,
        edited: m.edited_at > 0,
        deleted: m.deleted_at > 0,
        created_at: m.created_at,
        mine: m.sender_id == Some(me),
    }
}

pub async fn reply_index(
    pool: &SqlitePool,
    ids: &[i64],
) -> Result<std::collections::HashMap<i64, (String, String)>, sqlx::Error> {
    let mut map = std::collections::HashMap::new();
    if ids.is_empty() {
        return Ok(map);
    }
    let placeholders = vec!["?"; ids.len()].join(",");
    let q = format!(
        "SELECT m.id, m.body, u.name FROM messages m LEFT JOIN users u ON u.id = m.sender_id WHERE m.id IN ({})",
        placeholders
    );
    let mut query = sqlx::query_as::<_, (i64, String, Option<String>)>(&q);
    for id in ids {
        query = query.bind(*id);
    }
    for (id, body, name) in query.fetch_all(pool).await? {
        map.insert(id, (name.unwrap_or_default(), truncate(&body, 120)));
    }
    Ok(map)
}

// ------------------------------------------------------------------ files

pub async fn list_files(
    pool: &SqlitePool,
    parent: i64,
    trashed: bool,
) -> Result<Vec<FileNode>, sqlx::Error> {
    sqlx::query_as::<_, FileNode>(
        "SELECT * FROM files WHERE parent_id = ? AND trashed = ? ORDER BY kind DESC, name COLLATE NOCASE",
    )
    .bind(parent)
    .bind(if trashed { 1 } else { 0 })
    .fetch_all(pool)
    .await
}

pub async fn get_file(pool: &SqlitePool, id: i64) -> Result<Option<FileNode>, sqlx::Error> {
    sqlx::query_as::<_, FileNode>("SELECT * FROM files WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn file_by_name(
    pool: &SqlitePool,
    parent: i64,
    name: &str,
) -> Result<Option<FileNode>, sqlx::Error> {
    sqlx::query_as::<_, FileNode>(
        "SELECT * FROM files WHERE parent_id = ? AND name = ? COLLATE NOCASE AND trashed = 0 LIMIT 1",
    )
    .bind(parent)
    .bind(name)
    .fetch_optional(pool)
    .await
}

pub async fn child_count(pool: &SqlitePool, _id: i64) -> Result<i64, sqlx::Error> {
    let r = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM files WHERE parent_id = ? AND trashed = 0",
    )
    .fetch_one(pool)
    .await?;
    Ok(r.0)
}

#[allow(clippy::too_many_arguments)]
pub async fn create_node(
    pool: &SqlitePool,
    parent: i64,
    owner: i64,
    name: &str,
    mime: &str,
    size: i64,
    storage_key: &str,
    kind: &str,
) -> Result<FileNode, sqlx::Error> {
    let now = now_ms();
    let id = sqlx::query(
        "INSERT INTO files (parent_id, owner_id, name, mime, size, storage_key, kind, created_at, updated_at)
         VALUES (?,?,?,?,?,?,?,?,?)",
    )
    .bind(parent)
    .bind(owner)
    .bind(name)
    .bind(mime)
    .bind(size)
    .bind(storage_key)
    .bind(kind)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?
    .last_insert_rowid();
    get_file(pool, id).await.map(|o| o.unwrap())
}

pub async fn rename_node(
    pool: &SqlitePool,
    id: i64,
    name: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE files SET name = ?, updated_at = ? WHERE id = ?")
        .bind(name)
        .bind(now_ms())
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn move_node(pool: &SqlitePool, id: i64, parent: i64) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE files SET parent_id = ?, updated_at = ? WHERE id = ?")
        .bind(parent)
        .bind(now_ms())
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn trash_node(pool: &SqlitePool, id: i64, on: bool) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE files SET trashed = ?, updated_at = ? WHERE id = ?")
        .bind(if on { 1 } else { 0 })
        .bind(now_ms())
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn purge_node(pool: &SqlitePool, id: i64) -> Result<FileNode, sqlx::Error> {
    let node = get_file(pool, id).await?.ok_or(sqlx::Error::RowNotFound)?;
    // remove descendants
    let _ = sqlx::query("DELETE FROM files WHERE parent_id = ? AND trashed = 1")
        .bind(id)
        .execute(pool)
        .await;
    sqlx::query("DELETE FROM files WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(node)
}

pub async fn share_counts(pool: &SqlitePool, ids: &[i64]) -> Result<std::collections::HashMap<i64, i64>, sqlx::Error> {
    let mut map = std::collections::HashMap::new();
    if ids.is_empty() {
        return Ok(map);
    }
    let placeholders = vec!["?"; ids.len()].join(",");
    let q = format!("SELECT file_id, COUNT(*) FROM file_shares WHERE file_id IN ({}) GROUP BY file_id", placeholders);
    let mut query = sqlx::query_as::<_, (i64, i64)>(&q);
    for id in ids {
        query = query.bind(*id);
    }
    for (fid, n) in query.fetch_all(pool).await? {
        map.insert(fid, n);
    }
    Ok(map)
}

pub async fn set_share(
    pool: &SqlitePool,
    fid: i64,
    uid: i64,
    perm: &str,
    on: bool,
) -> Result<(), sqlx::Error> {
    if on {
        sqlx::query(
            "INSERT OR REPLACE INTO file_shares (file_id, user_id, permission) VALUES (?,?,?)",
        )
        .bind(fid)
        .bind(uid)
        .bind(perm)
        .execute(pool)
        .await?;
    } else {
        sqlx::query("DELETE FROM file_shares WHERE file_id = ? AND user_id = ?")
            .bind(fid)
            .bind(uid)
            .execute(pool)
            .await?;
    }
    Ok(())
}

pub async fn storage_used(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    let r = sqlx::query_as::<_, (i64,)>(
        "SELECT COALESCE(SUM(size),0) FROM files WHERE kind = 'file' AND trashed = 0",
    )
    .fetch_one(pool)
    .await?;
    Ok(r.0)
}

pub async fn search_files(
    pool: &SqlitePool,
    q: &str,
    limit: i64,
) -> Result<Vec<FileNode>, sqlx::Error> {
    let pat = format!("%{}%", q.replace('%', "\\%"));
    sqlx::query_as::<_, FileNode>(
        "SELECT * FROM files WHERE trashed = 0 AND name LIKE ? ESCAPE '\\' ORDER BY kind DESC, name COLLATE NOCASE LIMIT ?",
    )
    .bind(pat)
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn all_files(pool: &SqlitePool) -> Result<Vec<FileNode>, sqlx::Error> {
    sqlx::query_as::<_, FileNode>("SELECT * FROM files ORDER BY id").fetch_all(pool).await
}

// ------------------------------------------------------------------ calls

#[derive(FromRow)]
struct CallRow {
    id: i64,
    kind: String,
    conversation_id: Option<i64>,
    started_by: Option<i64>,
    status: String,
    started_at: i64,
    ended_at: i64,
}

pub async fn create_call(
    pool: &SqlitePool,
    kind: &str,
    cid: i64,
    by: i64,
) -> Result<Call, sqlx::Error> {
    let now = now_ms();
    let id = sqlx::query(
        "INSERT INTO calls (kind, conversation_id, started_by, status, started_at) VALUES (?,?,?,'active',?)",
    )
    .bind(kind)
    .bind(cid)
    .bind(by)
    .bind(now)
    .execute(pool)
    .await?
    .last_insert_rowid();
    sqlx::query(
        "INSERT INTO call_participants (call_id, user_id, joined_at) VALUES (?,?,?)",
    )
    .bind(id)
    .bind(by)
    .bind(now)
    .execute(pool)
    .await?;
    sqlx::query_as::<_, Call>("SELECT * FROM calls WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await
}

pub async fn get_call(pool: &SqlitePool, id: i64) -> Result<Option<Call>, sqlx::Error> {
    sqlx::query_as::<_, Call>("SELECT * FROM calls WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn end_call(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
    let now = now_ms();
    sqlx::query("UPDATE calls SET status = 'ended', ended_at = ? WHERE id = ? AND status = 'active'")
        .bind(now)
        .bind(id)
        .execute(pool)
        .await?;
    sqlx::query("UPDATE call_participants SET left_at = ? WHERE call_id = ? AND left_at = 0")
        .bind(now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn active_call(pool: &SqlitePool) -> Result<Option<Call>, sqlx::Error> {
    sqlx::query_as::<_, Call>("SELECT * FROM calls WHERE status = 'active' ORDER BY id DESC LIMIT 1")
        .fetch_optional(pool)
        .await
}

pub async fn join_call(pool: &SqlitePool, id: i64, uid: i64) -> Result<(), sqlx::Error> {
    let now = now_ms();
    sqlx::query(
        "INSERT INTO call_participants (call_id, user_id, joined_at) VALUES (?,?,?)
         ON CONFLICT(call_id, user_id) DO UPDATE SET left_at = 0",
    )
    .bind(id)
    .bind(uid)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn leave_call(pool: &SqlitePool, id: i64, uid: i64) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE call_participants SET left_at = ? WHERE call_id = ? AND user_id = ? AND left_at = 0")
        .bind(now_ms())
        .bind(id)
        .bind(uid)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn call_participants(
    pool: &SqlitePool,
    id: i64,
) -> Result<Vec<(i64, i64)>, sqlx::Error> {
    sqlx::query_as::<_, (i64, i64)>(
        "SELECT user_id, left_at FROM call_participants WHERE call_id = ?",
    )
    .bind(id)
    .fetch_all(pool)
    .await
}

pub async fn call_history(pool: &SqlitePool, limit: i64) -> Result<Vec<Call>, sqlx::Error> {
    sqlx::query_as::<_, Call>("SELECT * FROM calls ORDER BY id DESC LIMIT ?")
        .bind(limit)
        .fetch_all(pool)
        .await
}

// ------------------------------------------------------------------ backups

pub async fn create_backup(
    pool: &SqlitePool,
    name: &str,
    size: i64,
    path: &str,
) -> Result<BackupRow, sqlx::Error> {
    let id = sqlx::query("INSERT INTO backups (name, size, path, created_at) VALUES (?,?,?,?)")
        .bind(name)
        .bind(size)
        .bind(path)
        .bind(now_ms())
        .execute(pool)
        .await?
        .last_insert_rowid();
    sqlx::query_as::<_, BackupRow>("SELECT * FROM backups WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await
}

pub async fn list_backups(pool: &SqlitePool) -> Result<Vec<BackupRow>, sqlx::Error> {
    sqlx::query_as::<_, BackupRow>("SELECT * FROM backups ORDER BY id DESC")
        .fetch_all(pool)
        .await
}

pub async fn delete_backup(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM backups WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

// ------------------------------------------------------------------ admin stats

#[derive(FromRow)]
pub struct CountRow {
    pub n: i64,
}

pub async fn stats(pool: &SqlitePool) -> Result<std::collections::BTreeMap<String, i64>, sqlx::Error> {
    let mut m = std::collections::BTreeMap::new();
    let queries: &[(&str, &str)] = &[
        ("users", "SELECT COUNT(*) AS n FROM users"),
        ("active_users", "SELECT COUNT(*) AS n FROM users WHERE state = 'active'"),
        ("conversations", "SELECT COUNT(*) AS n FROM conversations"),
        ("messages", "SELECT COUNT(*) AS n FROM messages WHERE deleted_at = 0"),
        ("files", "SELECT COUNT(*) AS n FROM files WHERE kind = 'file' AND trashed = 0"),
        ("folders", "SELECT COUNT(*) AS n FROM files WHERE kind = 'folder' AND trashed = 0"),
        ("devices", "SELECT COUNT(*) AS n FROM devices"),
        ("approved_devices", "SELECT COUNT(*) AS n FROM devices WHERE approved = 1 AND revoked = 0"),
        ("pending_requests", "SELECT COUNT(*) AS n FROM devices WHERE approved = 0 AND revoked = 0"),
        ("calls", "SELECT COUNT(*) AS n FROM calls"),
        ("active_calls", "SELECT COUNT(*) AS n FROM calls WHERE status = 'active'"),
        ("reactions", "SELECT COUNT(*) AS n FROM reactions"),
        ("shares", "SELECT COUNT(*) AS n FROM file_shares"),
    ];
    for (k, q) in queries {
        let n = sqlx::query_as::<_, CountRow>(q)
            .fetch_one(pool)
            .await
            .map(|r| r.n)
            .unwrap_or(0);
        m.insert((*k).to_string(), n);
    }
    m.insert("storage_used", storage_used(pool).await?);
    Ok(m)
}

pub fn json_map(m: &std::collections::BTreeMap<String, i64>) -> Value {
    let mut o = serde_json::Map::new();
    for (k, v) in m.iter() {
        o.insert(k.clone(), Value::from(*v));
    }
    Value::Object(o)
}
