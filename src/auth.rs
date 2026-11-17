use actix_web::cookie::{Cookie, SameSite};
use actix_web::{web, HttpRequest, HttpResponse};
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::models::{Device, User};
use crate::state::AppState;
use crate::store;
use crate::util::*;

pub const COOKIE: &str = "room_device";
pub const COOKIE_MAX_AGE_SECS: i64 = 60 * 60 * 24 * 365;

pub struct Auth {
    pub user: User,
    pub device: Device,
}

/// Resolve the caller from its persistent device credential.
pub async fn resolve(req: &HttpRequest, state: &AppState) -> Option<Auth> {
    let token = req.cookie(COOKIE)?.value().to_string();
    if token.len() < 16 {
        return None;
    }
    let hash = hash_token(&token);
    let device = store::device_by_hash(&state.pool, &hash).await.ok().flatten()?;
    if device.approved != 1 || device.revoked == 1 {
        return None;
    }
    let user = store::get_user(&state.pool, device.user_id).await.ok()?;
    if user.state != "active" {
        return None;
    }
    let _ = store::touch_device(&state.pool, device.id).await;
    let _ = store::touch_user(&state.pool, user.id).await;
    Some(Auth { user, device })
}

pub fn device_cookie(token: &str) -> Cookie<'static> {
    Cookie::build(COOKIE, token.to_string())
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(actix_web::cookie::time::Duration::seconds(COOKIE_MAX_AGE_SECS))
        .finish()
}

pub fn clear_cookie() -> Cookie<'static> {
    Cookie::build(COOKIE, "")
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(actix_web::cookie::time::Duration::seconds(0))
        .finish()
}

pub fn public(path: &str) -> bool {
    matches!(
        path,
        "/gate" | "/api/request-access" | "/api/health" | "/api/server-info"
    ) || path.starts_with("/assets/")
}

fn clean_display_name(name: &str) -> String {
    let t: String = name
        .chars()
        .map(|c| if (c as u32) < 0x20 { ' ' } else { c })
        .collect();
    let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
    if t.is_empty() {
        "Guest".into()
    } else {
        t.chars().take(60).collect()
    }
}

#[derive(Deserialize)]
pub struct AccessForm {
    pub name: String,
    pub device: Option<String>,
}

fn client_hashes(req: &HttpRequest) -> Option<String> {
    req.cookie(COOKIE).map(|c| hash_token(c.value()))
}

/// Handle a connection request from a device that is not yet authorised.
pub async fn request_access(
    req: HttpRequest,
    state: web::Data<AppState>,
    form: web::Form<AccessForm>,
) -> HttpResponse {
    let name = clean_display_name(&form.name);
    if name.len() < 2 {
        return HttpResponse::BadRequest()
            .content_type("text/html")
            .body(crate::templates::gate::gate_page(&state, Some("Please enter your name.")).await);
    }

    let ip = client_ip(&req);
    let ua_name = form
        .device
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| device_name_from_ua(&req));
    let platform = platform_from_ua(&req);

    // Resume an existing device record when this browser already has a credential.
    let existing = match client_hashes(&req) {
        Some(hash) => store::device_by_hash(&state.pool, &hash).await.ok().flatten(),
        None => None,
    };

    let (device, user, token, approved) = if let Some(d) = existing {
        if d.revoked == 1 {
            return HttpResponse::BadRequest()
                .content_type("text/html")
                .body(crate::templates::gate::gate_page(
                    &state,
                    Some("This device credential was revoked. Ask the administrator for access."),
                )
                .await);
        }
        let u = match store::get_user(&state.pool, d.user_id).await {
            Ok(u) => u,
            Err(_) => return HttpResponse::InternalServerError().finish(),
        };
        let token = req.cookie(COOKIE).map(|c| c.value().to_string()).unwrap_or_default();
        let _ = store::touch_device(&state.pool, d.id).await;
        let approved = d.approved == 1;
        (d, u, token, approved)
    } else {
        let handle = make_handle(&state.pool, &name).await;
        let owner_exists = store::owner_count(&state.pool).await.unwrap_or(0) > 0;
        let role = if !owner_exists { "owner" } else { "member" };
        let user = match store::create_user(&state.pool, &name, &handle, role).await {
            Ok(u) => u,
            Err(_) => {
                return HttpResponse::InternalServerError()
                    .content_type("text/html")
                    .body(crate::templates::gate::gate_page(&state, Some("Could not register.")).await)
            }
        };
        let token = random_token();
        let approved = !owner_exists;
        let device = match store::create_device(
            &state.pool,
            user.id,
            &ua_name,
            &platform,
            &hash_token(&token),
            "",
            &ip,
            approved,
        )
        .await
        {
            Ok(d) => d,
            Err(_) => {
                return HttpResponse::InternalServerError()
                    .content_type("text/html")
                    .body(
                        crate::templates::gate::gate_page(&state, Some("Could not register device."))
                            .await,
                    )
            }
        };
        (device, user, token, approved)
    };

    if !approved {
        let _ = store::audit(
            &state.pool,
            user.id,
            &user.name,
            "device.request",
            "device",
            device.id,
            &format!("{} · {}", device.name, device.platform),
            &ip,
        )
        .await;
        let html = crate::templates::gate::waiting_page(&state, &user).await;
        return HttpResponse::Ok()
            .content_type("text/html")
            .cookie(device_cookie(&token))
            .body(html);
    }

    let _ = store::audit(
        &state.pool,
        user.id,
        &user.name,
        if device.approved_at > 0 {
            "device.resume"
        } else {
            "device.approve"
        },
        "device",
        device.id,
        &format!("{} · {}", device.name, device.platform),
        &ip,
    )
    .await;

    let html = crate::templates::gate::approved_page(&state, &user).await;
    HttpResponse::Ok()
        .content_type("text/html")
        .cookie(device_cookie(&token))
        .body(html)
}

/// Sign out: drop the local credential and deactivate the device record.
pub async fn logout(req: HttpRequest, state: web::Data<AppState>) -> HttpResponse {
    if let Some(auth) = resolve(&req, &state).await {
        let _ = store::revoke_device(&state.pool, auth.device.id).await;
        let _ = store::set_presence(&state.pool, auth.user.id, "offline").await;
        let _ = store::audit(
            &state.pool,
            auth.user.id,
            &auth.user.name,
            "device.signout",
            "device",
            auth.device.id,
            &auth.device.name,
            &client_ip(&req),
        )
        .await;
    }
    HttpResponse::Found()
        .append_header(("location", "/gate"))
        .cookie(clear_cookie())
        .finish()
}

pub async fn make_handle(pool: &SqlitePool, name: &str) -> String {
    let base: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect();
    let base = base.trim_matches('-');
    let base = if base.is_empty() { "member" } else { base };
    let mut candidate = base.to_string();
    let mut i = 2;
    loop {
        if store::user_by_handle(pool, &candidate)
            .await
            .ok()
            .flatten()
            .is_none()
        {
            return candidate;
        }
        candidate = format!("{}-{}", base, i);
        i += 1;
    }
}
