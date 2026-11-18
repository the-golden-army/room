use crate::auth::Auth;
use crate::models::*;
use crate::state::AppState;
use crate::store;

pub async fn build(state: &AppState, auth: &Auth, section: &str) -> Boot {
    let users = store::list_users(&state.pool).await.unwrap_or_default();
    let user_dtos: Vec<UserDto> = users.iter().map(UserDto::from).collect();
    let by_id: std::collections::HashMap<i64, String> =
        users.iter().map(|u| (u.id, u.name.clone())).collect();

    let devices = store::list_devices(&state.pool).await.unwrap_or_default();
    let device_dtos: Vec<DeviceDto> = devices
        .iter()
        .map(|d| DeviceDto {
            id: d.id,
            user_id: d.user_id,
            owner: by_id.get(&d.user_id).cloned().unwrap_or_default(),
            name: d.name.clone(),
            platform: d.platform.clone(),
            ip: d.ip.clone(),
            approved: d.approved == 1,
            revoked: d.revoked == 1,
            current: d.id == auth.device.id,
            requested_at: d.requested_at,
            approved_at: d.approved_at,
            last_seen_at: d.last_seen_at,
        })
        .collect();

    let settings = store::all_settings(&state.pool).await.unwrap_or_default();
    let pending = store::count_pending(&state.pool).await.unwrap_or(0);

    Boot {
        me: UserDto::from(&auth.user),
        users: user_dtos,
        devices: device_dtos,
        settings,
        server: ServerInfo {
            name: state.cfg.server_name.clone(),
            version: state.version.to_string(),
            uptime_ms: state.uptime_ms(),
            host: state.hostname.clone(),
            lan_hint: state.cfg.lan_ssid_hint.clone(),
            url: state.cfg.lan_url(),
        },
        is_admin: auth.user.role == "owner" || auth.user.role == "admin",
        section: section.to_string(),
        pending_requests: pending,
    }
}
