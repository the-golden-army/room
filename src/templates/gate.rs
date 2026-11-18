use crate::models::User;
use crate::state::AppState;
use crate::store;
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
{lan}
</main>
</body>
</html>"#,
        title = esc(title),
        name = esc(&state.cfg.server_name),
        logo = super::logo_mark("h-11 w-11 drop-shadow-sm"),
        body = body,
        host = esc(&state.hostname),
        net = esc(&net),
        lan = match state.cfg.lan_url() {
            Some(url) => format!(
                "<div class=\"text-center text-[11px] ink3\">On another device, open <span class=\"font-semibold ink2\">{}</span></div>",
                esc(&url)
            ),
            None => String::new(),
        },
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
    // Before the first owner exists nobody could approve anyone, so the first
    // device forms the owner without a code. Afterwards the code is the way
    // back in for the owner, and the way in for everyone else is approval.
    let claims = store::has_owner(&state.pool).await.unwrap_or(true);
    let code_available = !state.cfg.setup_code.is_empty();
    // Name the person who will approve you. "Ask the host" means nothing to a
    // first-time visitor who does not yet know who the host is.
    let owner = if claims {
        store::first_owner(&state.pool).await.ok().flatten()
    } else {
        None
    };
    let who_approves = match &owner {
        Some(o) => format!(
            "This ROOM server is private. Enter your name to ask {} for access — they approve new devices from ROOM's admin console on their own device.",
            esc(&o.name)
        ),
        None => "This ROOM server is private. Enter your name to ask the host for access.".to_string(),
    };
    let (heading, intro, submit, footnote, code_field) = if !claims {
        (
            "Set up ROOM",
            "You are the first device here, so this account becomes the host. Pick a name and you are in — you can approve everyone else afterwards.",
            "Create the host account",
            "This server has no accounts yet. The first device becomes the owner.",
            String::new(),
        )
    } else {
        (
            "Request access",
            who_approves.as_str(),
            "Send request",
            "Approval is asked once per device and stays valid across restarts. The host can revoke it at any time from the admin console.",
            if code_available {
                "<label class=\"block\"><span class=\"mb-1.5 block text-xs font-semibold ink2\">Setup code <span class=\"font-normal ink3\">(only if the host gave you one)</span></span><input class=\"input\" name=\"code\" maxlength=\"24\" placeholder=\"ABCD-EFGH\" autocomplete=\"off\" spellcheck=\"false\"></label>".to_string()
            } else {
                String::new()
            },
        )
    };
    let body = format!(
        r#"<div class="panel rounded-2xl p-5 shadow-soft">
  <h1 class="text-[15px] font-bold">{heading}</h1>
  <p class="mt-1 text-[13px] leading-relaxed ink2">{intro}</p>
  {err}
  <form method="post" action="/api/request-access" class="mt-4 space-y-3">
    <label class="block">
      <span class="mb-1.5 block text-xs font-semibold ink2">Your name</span>
      <input class="input" name="name" maxlength="60" placeholder="e.g. Nour Yahyaoui" required autocomplete="name">
    </label>
    {code_field}
    <input type="hidden" name="device" value="">
    <button class="btn btn-brand w-full" type="submit">{submit}</button>
  </form>
</div>
<div class="flex items-start gap-2.5 rounded-xl border line panel2 px-3.5 py-3">
  <span class="mt-0.5 ink3">{icon}</span>
  <div class="text-[12px] leading-relaxed ink2">{footnote}</div>
</div>"#,
        err = err,
        icon = super::icon("lock", "h-4 w-4"),
        heading = heading,
        intro = intro,
        code_field = code_field,
        submit = submit,
        footnote = footnote,
    );
    shell(if claims { "Request access" } else { "Set up ROOM" }, &body, state)
}

pub async fn waiting_page(state: &AppState, user: &User, device: &str) -> String {
    // Tell the visitor exactly who can approve them and where that happens.
    // "Waiting for the host" is useless if you cannot see that the host is the
    // person who set the server up, and that the approval happens in their
    // admin console.
    let owner = store::first_owner(&state.pool).await.ok().flatten();
    let admin_url = state
        .cfg
        .lan_url()
        .map(|u| format!("{u}/admin?tab=requests"))
        .unwrap_or_else(|| "/admin?tab=requests".to_string());
    let who = match &owner {
        Some(o) => format!(
            "Ask <span class=\"font-semibold ink\">{}</span> to open ROOM on their own device and approve you.",
            esc(&o.name)
        ),
        None => "Ask the person who set this server up to approve you.".to_string(),
    };
    let body = format!(
        r#"<div class="panel rounded-2xl p-5 shadow-soft">
  <div class="mx-auto mb-3 grid h-12 w-12 place-items-center rounded-2xl panel2 ink3">{icon}</div>
  <h1 class="text-center text-[15px] font-bold">Waiting for approval</h1>
  <p class="mt-1.5 text-center text-[13px] leading-relaxed ink2">Hi {name}, this device is not approved yet. Keep this page open — it checks again every few seconds and takes you in as soon as you are approved.</p>
  <div class="mt-4 rounded-xl panel2 px-3.5 py-3 text-[13px] leading-relaxed ink2">{who}</div>
  <div class="mt-3 flex items-center justify-center gap-2 text-xs ink3">{spin}<span>Waiting for approval…</span></div>
  <dl class="mt-4 space-y-2 border-t line pt-3 text-[12px]">
    <div class="flex items-baseline justify-between gap-3"><dt class="ink3">Your device</dt><dd class="font-semibold ink">{device}</dd></div>
    <div class="flex items-baseline justify-between gap-3"><dt class="ink3">Requested as</dt><dd class="font-semibold ink">{name2}</dd></div>
    <div class="flex items-baseline justify-between gap-3"><dt class="ink3">Approved by</dt><dd class="font-semibold ink">{approver}</dd></div>
  </dl>
  <div class="mt-3 border-t line pt-3 text-[11px] leading-relaxed ink3">
    The approval happens in the host's admin console. If you are the host, sign in on the device that already has access — or enter the setup code on the previous screen to add this one.
  </div>
</div>
<div class="flex items-start gap-2.5 rounded-xl border line panel2 px-3.5 py-3">
  <span class="mt-0.5 ink3">{lock}</span>
  <div class="text-[12px] leading-relaxed ink2">Nobody else on this network can read your messages until you are let in.<br><span class="ink3">{admin}</span></div>
</div>"#,
        icon = super::icon("clock", "h-6 w-6"),
        name = esc(&user.name),
        name2 = esc(&user.name),
        device = esc(device),
        who = who,
        approver = owner.as_ref().map(|o| esc(&o.name)).unwrap_or_else(|| "the host".to_string()),
        spin = super::spinner(""),
        lock = super::icon("lock", "h-4 w-4"),
        admin = esc(&admin_url),
    );
    let mut html = shell("Waiting for approval", &body, state);
    // Refresh a real page, not the form endpoint, so the browser never has to
    // re-POST and the visitor never sees the form again by accident.
    html = html.replace(
        "</head>",
        "<meta http-equiv=\"refresh\" content=\"5;url=/waiting\">\n</head>",
    );
    html
}

pub fn suspended_page(state: &AppState) -> String {
    let body = format!(
        r#"<div class="panel rounded-2xl p-5 shadow-soft text-center">
  <div class="mx-auto mb-3 grid h-12 w-12 place-items-center rounded-2xl bg-red-50 text-red-500 dark:bg-red-950/50">{icon}</div>
  <h1 class="text-[15px] font-bold">Account suspended</h1>
  <p class="mt-1.5 text-[13px] leading-relaxed ink2">Your account has been suspended by the host. Ask the administrator to restore it, then reload this page.</p>
  <a href="/gate" class="btn btn-ghost mt-4 w-full">Back to sign in</a>
</div>"#,
        icon = super::icon("warning", "h-6 w-6"),
    );
    shell("Suspended", &body, state)
}

/// Short "how long ago" label for a timestamp in ms since the epoch.
pub fn age(ts: i64) -> String {
    human_duration(chrono::Utc::now().timestamp_millis() - ts)
}
