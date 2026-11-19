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

/// Like `resolve`, but reports a credential that is otherwise valid whose account
/// the host has suspended. Used to explain the refusal instead of silently
/// treating the visitor as a stranger.
pub async fn suspended_user(req: &HttpRequest, state: &AppState) -> Option<User> {
    let token = req.cookie(COOKIE)?.value().to_string();
    if token.len() < 16 {
        return None;
    }
    let device = store::device_by_hash(&state.pool, &hash_token(&token))
        .await
        .ok()
        .flatten()?;
    if device.approved != 1 || device.revoked == 1 {
        return None;
    }
    let user = store::get_user(&state.pool, device.user_id).await.ok()?;
    if user.state == "active" {
        return None;
    }
    Some(user)
}

/// A device that registered but is still waiting for approval, together with
/// its user. Lets `/waiting` show the real state instead of re-offering the
/// form to somebody who already filled it in.
pub async fn pending_device(req: &HttpRequest, state: &AppState) -> Option<(crate::models::Device, User)> {
    let token = req.cookie(COOKIE)?.value().to_string();
    if token.len() < 16 {
        return None;
    }
    let device = store::device_by_hash(&state.pool, &hash_token(&token))
        .await
        .ok()
        .flatten()?;
    if device.approved == 1 || device.revoked == 1 {
        return None;
    }
    let user = store::get_user(&state.pool, device.user_id).await.ok()?;
    Some((device, user))
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
    /// Host-issued code, typed on the gate when nobody approved is left to
    /// approve you — the very first connection, or the owner's recovery.
    pub code: Option<String>,
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
        if u.state != "active" {
            return HttpResponse::Ok()
                .content_type("text/html")
                .body(crate::templates::gate::suspended_page(&state));
        }
        let mut approved = d.approved == 1;
        if !approved && state.is_open() {
            let _ = store::approve_device(&state.pool, d.id).await;
            approved = true;
        }
        (d, u, token, approved)
    } else {
        let handle = make_handle(&state.pool, &name).await;
        let owner_exists = store::has_owner(&state.pool).await.unwrap_or(true);
        // Nobody can approve anything until the first owner exists, so the first
        // device forms the owner with no code. After that a code is required to
        // mint a new owner — that is the recovery path when every approved
        // device is gone (lost laptop, cleared cookies, revoked by mistake).
        let given_code = form.code.as_deref().unwrap_or("").trim();
        let code_ok = !state.cfg.setup_code.is_empty()
            && crate::util::code_matches(&state.cfg.setup_code, given_code);
        if owner_exists && !given_code.is_empty() && !code_ok {
            return HttpResponse::BadRequest()
                .content_type("text/html")
                .body(
                    crate::templates::gate::gate_page(&state, Some("That setup code is not correct."))
                        .await,
                );
        }
        // With the host's code, reuse the existing owner instead of minting a
        // second one (the app deliberately supports a single owner). This is the
        // recovery path: it adds an approved device back to that account.
        let recovering = owner_exists && code_ok;
        let existing_owner = if recovering {
            store::first_owner(&state.pool).await.ok().flatten()
        } else {
            None
        };
        let role = if !owner_exists { "owner" } else { "member" };
        if let Some(u) = existing_owner {
            // Skip provisioning a user record entirely and continue with theirs.
            let token = random_token();
            let device = match store::create_device(
                &state.pool,
                u.id,
                &ua_name,
                &platform,
                &hash_token(&token),
                "",
                &ip,
                true,
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
            let _ = store::audit(
                &state.pool,
                u.id,
                &u.name,
                "device.recover",
                "device",
                device.id,
                &format!("{} · {}", device.name, device.platform),
                &ip,
            )
            .await;
            return HttpResponse::SeeOther()
                .append_header(("location", "/"))
                .cookie(device_cookie(&token))
                .finish();
        }
        let user = match store::create_user(&state.pool, &name, &handle, role).await {
            Ok(u) => u,
            Err(_) => {
                return HttpResponse::InternalServerError()
                    .content_type("text/html")
                    .body(crate::templates::gate::gate_page(&state, Some("Could not register.")).await)
            }
        };
        let token = random_token();
        // An open server lets anyone on the network in without a queue. The host
        // can still revoke a device afterwards, so this is convenience, not the
        // absence of a door.
        let approved = !owner_exists || state.is_open();
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
        // Redirect instead of rendering here: the form posts to an API path,
        // and leaving the browser on it made a refresh re-submit the form.
        return HttpResponse::SeeOther()
            .append_header(("location", "/waiting"))
            .cookie(device_cookie(&token))
            .finish();
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

    // Same reasoning: put the browser on the workspace, not on the API path.
    HttpResponse::SeeOther()
        .append_header(("location", "/"))
        .cookie(device_cookie(&token))
        .finish()
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
