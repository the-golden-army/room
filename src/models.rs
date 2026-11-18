use serde::Serialize;
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub handle: String,
    pub avatar: String,
    pub color: String,
    pub role: String,
    pub state: String,
    pub bio: String,
    pub created_at: i64,
    pub last_seen_at: i64,
    pub presence: String,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Device {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub platform: String,
    pub token_hash: String,
    pub fingerprint: String,
    pub ip: String,
    pub approved: i32,
    pub revoked: i32,
    pub requested_at: i64,
    pub approved_at: i64,
    pub last_seen_at: i64,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Conversation {
    pub id: i64,
    pub kind: String,
    pub name: String,
    pub topic: String,
    pub color: String,
    pub created_by: Option<i64>,
    pub created_at: i64,
    pub last_message_at: i64,
    pub last_preview: String,
    pub last_sender: String,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Message {
    pub id: i64,
    pub conversation_id: i64,
    pub sender_id: Option<i64>,
    pub body: String,
    pub reply_to_id: i64,
    pub edited_at: i64,
    pub deleted_at: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct FileNode {
    pub id: i64,
    pub parent_id: i64,
    pub owner_id: Option<i64>,
    pub name: String,
    pub mime: String,
    pub size: i64,
    pub storage_key: String,
    pub kind: String,
    pub trashed: i32,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Call {
    pub id: i64,
    pub kind: String,
    pub conversation_id: Option<i64>,
    pub started_by: Option<i64>,
    pub status: String,
    pub started_at: i64,
    pub ended_at: i64,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct AuditEntry {
    pub id: i64,
    pub actor_id: i64,
    pub actor_name: String,
    pub action: String,
    pub entity: String,
    pub entity_id: i64,
    pub detail: String,
    pub ip: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, FromRow)]
pub struct ReactionRow {
    pub message_id: i64,
    pub user_id: i64,
    pub emoji: String,
}

// Membership metadata (role, last-read cursor, mute) is part of the domain even
// though the UI currently reads only `user_id`; `SELECT *` in conversation_members
// maps by column name, so the fields must stay.
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct MemberRow {
    pub conversation_id: i64,
    pub user_id: i64,
    pub role: String,
    pub joined_at: i64,
    pub last_read_id: i64,
    pub muted: i32,
}

#[derive(Debug, Clone, FromRow)]
pub struct SettingRow {
    pub key: String,
    pub value: String,
}

// `path` is the on-disk file the row points at; deletion re-reads it by id, so the
// field is never read, but `SELECT *` mapping requires it.
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct BackupRow {
    pub id: i64,
    pub name: String,
    pub size: i64,
    pub path: String,
    pub created_at: i64,
}

// ---------------------------------------------------------------- DTOs

#[derive(Debug, Clone, Serialize)]
pub struct UserDto {
    pub id: i64,
    pub name: String,
    pub handle: String,
    pub avatar: String,
    pub color: String,
    pub initials: String,
    pub role: String,
    pub state: String,
    pub presence: String,
    pub bio: String,
    pub last_seen_at: i64,
    pub online: bool,
}

impl From<&User> for UserDto {
    fn from(u: &User) -> Self {
        UserDto {
            id: u.id,
            name: u.name.clone(),
            handle: u.handle.clone(),
            avatar: u.avatar.clone(),
            color: if u.color.is_empty() {
                crate::util::avatar_color(&u.handle)
            } else {
                u.color.clone()
            },
            initials: crate::util::initials(&u.name),
            role: u.role.clone(),
            state: u.state.clone(),
            presence: u.presence.clone(),
            bio: u.bio.clone(),
            last_seen_at: u.last_seen_at,
            online: u.presence == "online",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ConvDto {
    pub id: i64,
    pub kind: String,
    pub name: String,
    pub title: String,
    pub topic: String,
    pub color: String,
    pub initials: String,
    pub members: Vec<i64>,
    pub unread: i64,
    pub muted: bool,
    pub last_message_at: i64,
    pub last_preview: String,
    pub last_sender: String,
    pub my_last_read: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReactionDto {
    pub emoji: String,
    pub users: Vec<i64>,
    pub names: Vec<String>,
    #[serde(rename = "mine")]
    pub mine: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct MsgDto {
    pub id: i64,
    pub conversation_id: i64,
    pub sender_id: i64,
    pub sender_name: String,
    pub sender_avatar: String,
    pub sender_color: String,
    pub sender_initials: String,
    pub body: String,
    pub clean: String,
    pub reply_to_id: i64,
    pub reply_to_name: String,
    pub reply_to_body: String,
    pub reactions: Vec<ReactionDto>,
    pub edited: bool,
    pub deleted: bool,
    pub created_at: i64,
    pub mine: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileDto {
    pub id: i64,
    pub parent_id: i64,
    pub name: String,
    pub mime: String,
    pub size: i64,
    pub kind: String,
    pub trashed: bool,
    pub owner_id: i64,
    pub owner_name: String,
    pub shared: bool,
    pub children: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CallDto {
    pub id: i64,
    pub kind: String,
    pub status: String,
    pub started_by: i64,
    pub starter_name: String,
    pub conversation_id: i64,
    pub participants: Vec<i64>,
    pub participant_names: Vec<String>,
    pub started_at: i64,
    pub ended_at: i64,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeviceDto {
    pub id: i64,
    pub user_id: i64,
    pub owner: String,
    pub name: String,
    pub platform: String,
    pub ip: String,
    pub approved: bool,
    pub revoked: bool,
    pub current: bool,
    pub requested_at: i64,
    pub approved_at: i64,
    pub last_seen_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditDto {
    pub id: i64,
    pub actor_name: String,
    pub action: String,
    pub entity: String,
    pub detail: String,
    pub ip: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BackupDto {
    pub id: i64,
    pub name: String,
    pub size: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
    pub uptime_ms: i64,
    pub host: String,
    pub lan_hint: String,
    /// Address other devices on the same network can open, when detectable.
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Boot {
    pub me: UserDto,
    pub users: Vec<UserDto>,
    pub devices: Vec<DeviceDto>,
    pub settings: std::collections::BTreeMap<String, String>,
    pub server: ServerInfo,
    pub is_admin: bool,
    pub section: String,
    pub pending_requests: i64,
}
