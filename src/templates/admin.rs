use crate::models::{AuditDto, BackupDto, Boot, DeviceDto, ServerInfo};
use crate::util::{clock, esc, human_bytes, human_duration};

use super::{avatar, empty_state, icon, topbar};

pub const TABS: &[(&str, &str, &str)] = &[
    ("overview", "Overview", "activity"),
    ("requests", "Requests", "bell"),
    ("devices", "Devices", "monitor"),
    ("users", "Users", "users"),
    ("audit", "Audit log", "list"),
    ("backups", "Backups", "archive"),
    ("config", "Configuration", "settings"),
];

pub fn list_column(boot: &Boot, tab: &str, pending: i64) -> String {
    let mut links = String::new();
    for (key, label, ic) in TABS {
        let active = if *key == tab { " active" } else { "" };
        let badge = if *key == "requests" && pending > 0 {
            format!(
                "<span class=\"ml-auto grid h-[18px] min-w-[18px] place-items-center rounded-full px-1 text-[9px] font-bold text-white\" style=\"background:#ef4444\">{}</span>",
                pending
            )
        } else {
            String::new()
        };
        links.push_str(&format!(
            "<a class=\"navlink{active}\" href=\"/admin?tab={key}\" data-tab=\"{key}\">{}<span class=\"flex-1\">{}</span>{badge}</a>",
            icon(ic, "h-4 w-4"),
            esc(label),
            badge = badge
        ));
    }
    let _ = boot;
    format!(
        "{header}<div class=\"flex flex-col gap-0.5 p-2\">{links}</div>",
        header = super::listcol_header("Server", ""),
        links = links
    )
}

fn card(label: &str, value: &str, ic: &str, sub: &str) -> String {
    format!(
        "<div class=\"tile p-4\"><div class=\"flex items-center gap-2 text-[11px] font-semibold uppercase tracking-[0.05em] ink3\">{} {}</div>\
<div class=\"mt-2 text-2xl font-extrabold tracking-tight\">{}</div><div class=\"mt-0.5 text-[11px] ink3\">{}</div></div>",
        icon(ic, "h-3.5 w-3.5"),
        esc(label),
        esc(value),
        esc(sub)
    )
}

pub fn overview(
    boot: &Boot,
    stats: &std::collections::BTreeMap<String, i64>,
    audit: &[AuditDto],
    g: &str,
) -> String {
    let s = |k: &str| stats.get(k).copied().unwrap_or(0);
    let cards = format!(
        "<div class=\"grid grid-cols-2 gap-3 p-4 lg:grid-cols-3\">\
{}{}{}{}{}{}</div>",
        card("Messages", &s("messages").to_string(), "chat", &format!("{} conversations", s("conversations"))),
        card("Files", &s("files").to_string(), "folder", &format!("{} folders", s("folders"))),
        card("Storage", &human_bytes(s("storage_used")), "drive", "on this machine"),
        card("Members", &s("users").to_string(), "users", &format!("{} online now", s("active_users"))),
        card("Devices", &s("approved_devices").to_string(), "monitor", &format!("{} pending", s("pending_requests"))),
        card("Calls", &s("calls").to_string(), "phone", &format!("{} active", s("active_calls"))),
    );
    let mut activity = String::new();
    if audit.is_empty() {
        activity.push_str(&empty_state("activity", "No activity yet", "Actions taken on this server will appear here.", ""));
    } else {
        for a in audit.iter().take(9) {
            activity.push_str(&audit_row(a));
        }
    }
    let status = server_status(boot.server.clone(), boot);
    let body = format!(
        "{cards}\
<div class=\"grid gap-3 px-4 pb-4 lg:grid-cols-2\">\
<div class=\"tile p-4\"><div class=\"mb-3 flex items-center gap-2 text-[13px] font-bold\">{} Server status</div>{status}</div>\
<div class=\"tile p-4\"><div class=\"mb-3 flex items-center gap-2 text-[13px] font-bold\">{} Recent activity</div><div class=\"flex flex-col\">{activity}</div></div>\
</div>",
        icon("server", "h-4 w-4"),
        icon("activity", "h-4 w-4")
    );
    let _ = g;
    format!(
        "{}{}",
        topbar("", "Overview", "Host console", ""),
        super::scroll_area("admin-scroll", &body, "")
    )
}

fn server_status(info: ServerInfo, boot: &Boot) -> String {
    let rows = [
        ("Uptime", clock(info.uptime_ms)),
        ("Hostname", info.host.clone()),
        ("Version", format!("ROOM {}", info.version)),
        ("Members online", boot.users.iter().filter(|u| u.online).count().to_string()),
        ("Pending approvals", boot.pending_requests.to_string()),
        ("Audit entries", boot.devices.len().to_string()),
    ];
    let mut out = String::new();
    for (k, v) in rows {
        out.push_str(&format!(
            "<div class=\"flex items-center justify-between border-b line py-2 text-[12.5px]\"><span class=\"ink3\">{}</span><span class=\"font-semibold\">{}</span></div>",
            esc(k),
            esc(&v)
        ));
    }
    out.push_str(&format!(
        "<div class=\"mt-3 flex flex-wrap gap-2\">\
<button class=\"btn btn-ghost btn-xs\" data-act=\"backup-now\" type=\"button\">{} Create backup</button>\
<button class=\"btn btn-ghost btn-xs\" data-act=\"restart\" type=\"button\">{} Restart server</button>\
<button class=\"btn btn-ghost btn-xs hover:!text-red-500\" data-act=\"shutdown\" type=\"button\">{} Shut down</button>\
</div>",
        icon("archive", "h-3.5 w-3.5"),
        icon("refresh", "h-3.5 w-3.5"),
        icon("power", "h-3.5 w-3.5")
    ));
    out
}

fn audit_row(a: &AuditDto) -> String {
    let (ic, color) = match a.action.as_str() {
        "device.approve" | "device.resume" => ("check", "#10b981"),
        "device.request" => ("bell", "#f59e0b"),
        "device.revoke" | "device.signout" => ("logout", "#ef4444"),
        "message.send" => ("chat", "#5333f4"),
        "file.upload" => ("upload", "#0ea5e9"),
        "file.delete" => ("trash", "#ef4444"),
        "call.start" => ("phone", "#8b5cf6"),
        "user.update" => ("user", "#14b8a6"),
        _ => ("activity", "#64748b"),
    };
    format!(
        "<div class=\"flex items-center gap-3 border-b line py-2 last:border-0\">\
<span class=\"grid h-8 w-8 shrink-0 place-items-center rounded-lg\" style=\"color:{color};background:color-mix(in srgb, {color} 12%, transparent)\">{}</span>\
<div class=\"min-w-0 flex-1\"><div class=\"truncate text-[12.5px] font-medium\">{}</div><div class=\"truncate text-[11px] ink3\">{} · {}</div></div>\
<span class=\"shrink-0 text-[10.5px] ink3\">{time}</span></div>",
        icon(ic, "h-4 w-4"),
        esc(&a.detail),
        esc(&a.actor_name),
        esc(&a.action),
        time = human_duration(crate::util::now_ms() - a.created_at)
    )
}

pub fn requests(boot: &Boot, pending: &[DeviceDto]) -> String {
    let body = if pending.is_empty() {
        empty_state(
            "bell",
            "No pending requests",
            "When someone on the network opens this server, their request shows up here.",
            "",
        )
    } else {
        let mut out = String::new();
        for d in pending {
            out.push_str(&format!(
                "<div class=\"tile p-3.5\">\
<div class=\"flex items-center gap-3\">{}<div class=\"min-w-0 flex-1\"><div class=\"truncate text-[13.5px] font-semibold\">{}</div><div class=\"truncate text-[11.5px] ink3\">{} · {} · {} · requested {}</div></div></div>\
<div class=\"mt-3 flex gap-2\">\
<button class=\"btn btn-brand btn-xs\" data-approve=\"{}\" type=\"button\">{} Approve</button>\
<button class=\"btn btn-danger btn-xs\" data-reject=\"{}\" type=\"button\">{} Reject</button>\
</div></div>",
                icon("smartphone", "h-5 w-5"),
                esc(&d.name),
                esc(&d.owner),
                esc(&d.platform),
                esc(&d.ip),
                super::gate::age(d.requested_at),
                d.id,
                icon("check", "h-3.5 w-3.5"),
                d.id,
                icon("x", "h-3.5 w-3.5")
            ));
        }
        format!("<div class=\"grid gap-2.5 p-4\">{}</div>", out)
    };
    let _ = boot;
    format!(
        "{}{}",
        topbar("", "Access requests", &format!("{} waiting", pending.len()), ""),
        super::scroll_area("admin-scroll", &body, "")
    )
}

pub fn devices(boot: &Boot) -> String {
    let mut rows = String::new();
    for d in &boot.devices {
        let state = if d.revoked {
            "<span class=\"chip\" style=\"background:color-mix(in srgb,#ef4444 12%,transparent);color:#dc2626\">revoked</span>"
        } else if d.approved {
            "<span class=\"chip\" style=\"background:color-mix(in srgb,#10b981 12%,transparent);color:#059669\">approved</span>"
        } else {
            "<span class=\"chip\" style=\"background:color-mix(in srgb,#f59e0b 12%,transparent);color:#d97706\">pending</span>"
        };
        rows.push_str(&format!(
            "<div class=\"tile flex items-center gap-3 p-3\">\
{}<div class=\"min-w-0 flex-1\"><div class=\"flex items-center gap-2\"><span class=\"truncate text-[13px] font-semibold\">{}</span>{}{}</div>\
<div class=\"truncate text-[11px] ink3\">{} · {} · last seen {}</div></div>\
<button class=\"btn btn-ghost btn-xs hover:!text-red-500\" data-revoke=\"{}\" type=\"button\">{}</button></div>",
            icon("monitor", "h-5 w-5"),
            esc(&d.name),
            state,
            if d.current { "<span class=\"chip chip-brand\">this device</span>" } else { "" },
            esc(&d.owner),
            esc(&d.platform),
            if d.last_seen_at > 0 { human_duration(crate::util::now_ms() - d.last_seen_at) } else { "never".into() },
            d.id,
            icon("logout", "h-3.5 w-3.5")
        ));
    }
    let body = if boot.devices.is_empty() {
        empty_state("monitor", "No devices", "Approved devices appear here.", "")
    } else {
        format!("<div class=\"grid gap-2 p-4\">{}</div>", rows)
    };
    format!(
        "{}{}",
        topbar("", "Devices", &format!("{} registered", boot.devices.len()), ""),
        super::scroll_area("admin-scroll", &body, "")
    )
}

pub fn users(boot: &Boot, me: i64) -> String {
    let mut rows = String::new();
    for u in &boot.users {
        let role = format!(
            "<select class=\"input\" style=\"width:auto;padding:.3rem .5rem;font-size:.75rem\" data-user-role=\"{}\">\
<option value=\"member\" {}>member</option><option value=\"admin\" {}>admin</option><option value=\"owner\" {}>owner</option></select>",
            u.id,
            if u.role == "member" { "selected" } else { "" },
            if u.role == "admin" { "selected" } else { "" },
            if u.role == "owner" { "selected" } else { "" }
        );
        let state = if u.id == me {
            "<span class=\"chip chip-brand\">you</span>".to_string()
        } else if u.state == "suspended" {
            format!(
                "<button class=\"btn btn-ghost btn-xs\" data-user-unsuspend=\"{}\" type=\"button\">{}</button>",
                u.id,
                icon("restore", "h-3.5 w-3.5")
            )
        } else {
            format!(
                "<button class=\"btn btn-ghost btn-xs hover:!text-red-500\" data-user-suspend=\"{}\" type=\"button\">{}</button>",
                u.id,
                icon("lock", "h-3.5 w-3.5")
            )
        };
        rows.push_str(&format!(
            "<div class=\"tile flex items-center gap-3 p-3\">{}<div class=\"min-w-0 flex-1\"><div class=\"truncate text-[13px] font-semibold\">{}</div><div class=\"truncate text-[11px] ink3\">@\"{}\" · joined {}</div></div>{}{}</div>",
            avatar(u, "h-9 w-9 text-xs", true),
            esc(&u.name),
            esc(&u.handle),
            human_duration(crate::util::now_ms() - u.last_seen_at),
            role,
            state
        ));
    }
    format!(
        "{}{}",
        topbar("", "Members", &format!("{} people", boot.users.len()), ""),
        super::scroll_area("admin-scroll", &format!("<div class=\"grid gap-2 p-4\">{}</div>", rows), "")
    )
}

pub fn audit(boot: &Boot, entries: &[AuditDto]) -> String {
    let mut rows = String::new();
    for a in entries {
        rows.push_str(&format!(
            "<div class=\"tile flex items-center gap-3 p-3\">\
<div class=\"min-w-0 flex-1\"><div class=\"flex flex-wrap items-center gap-2\"><span class=\"text-[12.5px] font-semibold\">{}</span><span class=\"chip\">{}</span><span class=\"text-[11px] ink3\">{}</span></div><div class=\"mt-0.5 truncate text-[11.5px] ink2\">{}</div></div>\
<span class=\"shrink-0 text-[10.5px] ink3\">{}</span></div>",
            esc(&a.actor_name),
            esc(&a.action),
            esc(&a.entity),
            esc(&a.detail),
            human_duration(crate::util::now_ms() - a.created_at)
        ));
    }
    let _ = boot;
    let body = if entries.is_empty() {
        empty_state("list", "Nothing logged yet", "Audit entries are recorded for every sensitive action.", "")
    } else {
        format!("<div class=\"grid gap-1.5 p-4\">{}</div>", rows)
    };
    format!(
        "{}{}",
        topbar("", "Audit log", &format!("{} entries", entries.len()), ""),
        super::scroll_area("admin-scroll", &body, "")
    )
}

pub fn backups(view: &BackupsView) -> String {
    let mut rows = String::new();
    for b in &view.items {
        rows.push_str(&format!(
            "<div class=\"tile flex items-center gap-3 p-3\">{}<div class=\"min-w-0 flex-1\"><div class=\"truncate text-[13px] font-semibold\">{}</div><div class=\"text-[11px] ink3\">{} · {}</div></div>\
<button class=\"btn btn-ghost btn-xs hover:!text-red-500\" data-backup-delete=\"{}\" type=\"button\">{}</button></div>",
            icon("archive", "h-5 w-5"),
            esc(&b.name),
            human_bytes(b.size),
            human_duration(crate::util::now_ms() - b.created_at),
            b.id,
            icon("trash", "h-3.5 w-3.5")
        ));
    }
    let body = format!(
        "<div class=\"grid gap-2 p-4\"><button class=\"btn btn-brand\" data-act=\"backup-now\" type=\"button\">{} Create backup now</button>{}</div>",
        icon("archive", "h-4 w-4"),
        if view.items.is_empty() {
            empty_state("archive", "No backups yet", "A backup copies the database and the file index into the data directory.", "")
        } else {
            rows
        }
    );
    format!(
        "{}{}",
        topbar("", "Backups", &format!("{}", view.items.len()), ""),
        super::scroll_area("admin-scroll", &body, "")
    )
}

pub struct BackupsView {
    pub items: Vec<BackupDto>,
}

pub fn config(boot: &Boot, msg: &str) -> String {
    let g = |k: &str| boot.settings.get(k).cloned().unwrap_or_default();
    let notice = if msg.is_empty() {
        String::new()
    } else {
        format!(
            "<div class=\"rounded-xl border p-3 text-[12.5px] font-medium\" style=\"border-color:color-mix(in srgb,#10b981 40%,transparent);background:color-mix(in srgb,#10b981 8%,transparent);color:#059669\">{}</div>",
            esc(msg)
        )
    };
    let f1 = super::field(
        "Server name",
        &format!(
            "<input class=\"input\" name=\"server_name\" value=\"{}\" maxlength=\"40\">",
            esc(&g("server_name"))
        ),
        "Shown in the header and on the sign-in screen."
    );
    let f2 = super::field(
        "Network name",
        &format!(
            "<input class=\"input\" name=\"network_name\" value=\"{}\" maxlength=\"60\" placeholder=\"e.g. Home Wi-Fi\">",
            esc(&g("network_name"))
        ),
        "Hint shown on the request-access screen."
    );
    let f3 = super::field(
        "Upload limit (MB)",
        &format!(
            "<input class=\"input\" name=\"max_upload_mb\" type=\"number\" min=\"1\" max=\"102400\" value=\"{}\">",
            esc(&g("max_upload_mb"))
        ),
        "Largest single file a device may upload."
    );
    let f4 = super::field(
        "Default theme",
        &format!(
            "<select class=\"input\" name=\"theme\"><option value=\"light\" {}>Light</option><option value=\"dark\" {}>Dark</option><option value=\"system\" {}>Follow system</option></select>",
            if g("theme") == "light" { "selected" } else { "" },
            if g("theme") == "dark" { "selected" } else { "" },
            if g("theme").is_empty() || g("theme") == "system" { "selected" } else { "" }
        ),
        "Applied to new devices."
    );
    let body = format!(
        "<div class=\"grid gap-3 p-4\">{notice}\
<form class=\"tile space-y-4 p-5\" id=\"config-form\" method=\"dialog\">\
<div class=\"text-[13px] font-bold\">General</div>\
<div class=\"grid gap-3 sm:grid-cols-2\">{f1} {f2}</div>\
<div class=\"text-[13px] font-bold pt-1\">Limits</div>\
<div class=\"grid gap-3 sm:grid-cols-2\">{f3} {f4}</div>\
<div class=\"flex items-center justify-between gap-3 border-t line pt-4\">\
<div class=\"text-[11.5px] ink3\">Settings are stored in the local database and apply immediately.</div>\
<button class=\"btn btn-brand\" type=\"submit\">Save changes</button></div></form></div>",
        notice = notice,
        f1 = f1,
        f2 = f2,
        f3 = f3,
        f4 = f4
    );
    format!(
        "{}{}",
        topbar("", "Configuration", "Server settings", ""),
        super::scroll_area("admin-scroll", &body, "")
    )
}
