use crate::models::User;
use crate::state::AppState;
use crate::util::{esc, human_duration};

fn shell(title: &str, body: &str, state: &AppState) -> String {
    let net = if state.cfg.lan_ssid_hint.is_empty() {
        "this local network".to_string()
    } else {
        format!("“{}”", state.cfg.lan_ssid_hint)
    };
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
<title>{title} · {name}</title>
<link rel="icon" href="/favicon.svg" type="image/svg+xml">
<link rel="stylesheet" href="/assets/app.css">
</head>
<body class="min-h-dvh">
<main class="mx-auto flex min-h-dvh w-full max-w-md flex-col justify-center gap-6 px-6 py-10">
<div class="flex items-center gap-3">
  {logo}
  <div>
    <div class="text-lg font-extrabold tracking-tight">{name}</div>
    <div class="text-xs ink3">Private · Local · Fast</div>
  </div>
</div>
{body}
<div class="text-center text-[11px] ink3">Server <span class="font-semibold ink2">{host}</span> · {net}</div>
</main>
</body>
</html>"#,
        title = esc(title),
        name = esc(&state.cfg.server_name),
        logo = super::logo_mark("h-11 w-11 drop-shadow-sm"),
        body = body,
        host = esc(&state.hostname),
        net = esc(&net),
    )
}

pub async fn gate_page(state: &AppState, error: Option<&str>) -> String {
    let err = match error {
        Some(e) => format!(
            "<div class=\"rounded-xl border border-red-200 bg-red-50 px-3.5 py-2.5 text-[13px] font-medium text-red-700 dark:border-red-900/60 dark:bg-red-950/40 dark:text-red-300\">{}</div>",
            esc(e)
        ),
        None => String::new(),
    };
    let body = format!(
        r#"<div class="panel rounded-2xl p-5 shadow-soft">
  <h1 class="text-[15px] font-bold">Request access</h1>
  <p class="mt-1 text-[13px] leading-relaxed ink2">This ROOM server is private. Enter your name to ask the host for access — the device must be approved before you can open the workspace.</p>
  {err}
  <form method="post" action="/api/request-access" class="mt-4 space-y-3">
    <label class="block">
      <span class="mb-1.5 block text-xs font-semibold ink2">Your name</span>
      <input class="input" name="name" maxlength="60" placeholder="e.g. Nour Yahyaoui" required autocomplete="name">
    </label>
    <input type="hidden" name="device" value="">
    <button class="btn btn-brand w-full" type="submit">Send request</button>
  </form>
</div>
<div class="flex items-start gap-2.5 rounded-xl border line panel2 px-3.5 py-3">
  <span class="mt-0.5 ink3">{icon}</span>
  <div class="text-[12px] leading-relaxed ink2">Approval is asked once per device and stays valid across restarts. The host can revoke it at any time from the admin console.</div>
</div>"#,
        err = err,
        icon = super::icon("lock", "h-4 w-4"),
    );
    shell("Request access", &body, state)
}

pub async fn waiting_page(state: &AppState, user: &User) -> String {
    let body = format!(
        r#"<div class="panel rounded-2xl p-5 shadow-soft text-center">
  <div class="mx-auto mb-3 grid h-12 w-12 place-items-center rounded-2xl panel2 ink3">{icon}</div>
  <h1 class="text-[15px] font-bold">Request sent</h1>
  <p class="mt-1.5 text-[13px] leading-relaxed ink2">Hi {name}, your device <span class="font-semibold ink">{device}</span> is waiting for the host to approve it. This page will refresh automatically.</p>
  <div class="mt-4 flex items-center justify-center gap-2 text-xs ink3">{spin}<span>Waiting for approval…</span></div>
  <div class="mt-4 border-t line pt-3 text-[11px] ink3">Signed in as {handle}</div>
</div>"#,
        icon = super::icon("clock", "h-6 w-6"),
        name = esc(&user.name),
        device = esc(&user.handle),
        spin = super::spinner(""),
        handle = esc(&user.handle),
    );
    let mut html = shell("Waiting for approval", &body, state);
    html = html.replace(
        "</head>",
        "<meta http-equiv=\"refresh\" content=\"6\">\n</head>",
    );
    html
}

pub async fn approved_page(state: &AppState, user: &User) -> String {
    let body = format!(
        r#"<div class="panel rounded-2xl p-5 shadow-soft text-center">
  <div class="mx-auto mb-3 grid h-12 w-12 place-items-center rounded-2xl" style="background:color-mix(in srgb, var(--color-brand-600) 12%, transparent);color:var(--color-brand-600)">{icon}</div>
  <h1 class="text-[15px] font-bold">Welcome, {name}</h1>
  <p class="mt-1.5 text-[13px] ink2">Your device is authorised. Taking you to the workspace…</p>
  <a href="/" class="btn btn-brand mt-4 w-full">Open ROOM</a>
</div>"#,
        icon = super::icon("check", "h-6 w-6"),
        name = esc(&user.name),
    );
    let mut html = shell("Approved", &body, state);
    html = html.replace(
        "</head>",
        "<meta http-equiv=\"refresh\" content=\"1;url=/\">\n</head>",
    );
    html
}

pub fn suspended_page(state: &AppState) -> String {
    let body = format!(
        r#"<div class="panel rounded-2xl p-5 shadow-soft text-center">
  <div class="mx-auto mb-3 grid h-12 w-12 place-items-center rounded-2xl bg-red-50 text-red-500 dark:bg-red-950/50">{icon}</div>
  <h1 class="text-[15px] font-bold">Account suspended</h1>
  <p class="mt-1.5 text-[13px] ink2">Your account has been suspended by the host. Contact the administrator if you think this is a mistake.</p>
</div>"#,
        icon = super::icon("warning", "h-6 w-6"),
    );
    shell("Suspended", &body, state)
}

pub fn age(ms: i64) -> String {
    human_duration(ms)
}
