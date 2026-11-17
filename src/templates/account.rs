use crate::models::Boot;
use crate::util::{esc, human_duration};

use super::{avatar, icon, listcol_header, topbar};

pub fn list_column(boot: &Boot) -> String {
    format!(
        "{}{}",
        listcol_header("Account", ""),
        format!(
            "<div class=\\"flex flex-col gap-0.5 p-2\\">\
<a class=\\"navlink\\" href=\\"/account\\">{}<span>Profile</span></a>\
<a class=\\"navlink\\" href=\\"/account?tab=devices\\">{}<span>My devices</span></a>\
<a class=\\"navlink\\" href=\\"/account?tab=security\\">{}<span>Security</span></a>\
</div>",
            icon("user", "h-4 w-4"),
            icon("monitor", "h-4 w-4"),
            icon("lock", "h-4 w-4")
        )
    )
}

pub fn main_column(boot: &Boot, tab: &str) -> String {
    let me = &boot.me;
    let body = match tab {
        "devices" => {
            let mut rows = String::new();
            for d in boot.devices.iter().filter(|d| d.user_id == me.id) {
                let state = if d.revoked {
                    "<span class=\\"chip\\" style=\\"background:color-mix(in srgb,#ef4444 12%,transparent);color:#dc2626\\">revoked</span>"
                } else if d.approved {
                    "<span class=\\"chip\\" style=\\"background:color-mix(in srgb,#10b981 12%,transparent);color:#059669\\">approved</span>"
                } else {
                    "<span class=\\"chip\\" style=\\"background:color-mix(in srgb,#f59e0b 12%,transparent);color:#d97706\\">pending</span>"
                };
                rows.push_str(&format!(
                    "<div class=\\"tile flex items-center gap-3 p-3\\">{}<div class=\\"min-w-0 flex-1\\"><div class=\\"flex items-center gap-2\\"><span class=\\"truncate text-[13px] font-semibold\\">{}</span>{}{}</div><div class=\\"truncate text-[11px] ink3\\">{} · last seen {}</div></div>{}</div>",
                    icon("monitor", "h-5 w-5"),
                    esc(&d.name),
                    state,
                    if d.current { "<span class=\\"chip chip-brand\\">this device</span>" } else { "" },
                    esc(&d.platform),
                    if d.last_seen_at > 0 { human_duration(crate::util::now_ms() - d.last_seen_at) } else { "never".into() },
                    if d.current || d.revoked { String::new() } else { format!("<button class=\\"btn btn-ghost btn-xs hover:!text-red-500\\" data-revoke=\\"{}\\" type=\\"button\\">{}</button>", d.id, icon("logout", "h-3.5 w-3.5")) }
                ));
            }
            format!(
                "<div class=\\"grid gap-2 p-4\\">{}</div>",
                if boot.devices.iter().any(|d| d.user_id == me.id) {
                    rows
                } else {
                    super::empty_state("monitor", "No devices", "You have no other authorised devices.", "")
                }
            )
        }
        "security" => format!(
            "<div class=\\"grid gap-3 p-4\\">\
<div class=\\"tile p-5\\"><div class=\\"flex items-center gap-2 text-[13px] font-bold\\">{} How access works</div>\
<ul class=\\"mt-3 space-y-2.5 text-[12.5px] leading-relaxed ink2\\">\
<li>· Approval is required once per device. This browser keeps a long-lived credential.</li>\
<li>· Credentials are stored as salted SHA-256 hashes — never in clear text.</li>\
<li>· The host can revoke this device at any time; the next request is refused immediately.</li>\
<li>· A device identity is independent of your IP address.</li></ul></div>\
<div class=\\"tile p-5\\"><div class=\\"text-[13px] font-bold\\">{} Sign out</div><p class=\\"mt-1.5 text-[12.5px] ink2\\">Sign out revokes this device credential and clears the session.</p>\
<a class=\\"btn btn-danger mt-3\\" href=\\"/api/logout\\">{} Sign out this device</a></div></div>",
            icon("shield", "h-4 w-4"),
            icon("logout", "h-4 w-4"),
            icon("logout", "h-4 w-4")
        ),
        _ => format!(
            "<div class=\\"mx-auto w-full max-w-lg px-4 py-6\\">\
<div class=\\"panel rounded-2xl p-5 shadow-soft\\">\
<div class=\\"flex items-center gap-3\\">{}<div><div class=\\"text-[15px] font-bold\\">{}</div><div class=\\"text-xs ink3\\">@{} · {}</div></div></div>\
<form id=\\"profile-form\\" class=\\"mt-5 space-y-4\\">\
{} {} \
<button class=\\"btn btn-brand\\" type=\\"submit\\">Save profile</button></form></div></div>",
            avatar(me, "h-16 w-16 text-xl", true),
            esc(&me.name),
            esc(&me.handle),
            if me.role == "owner" { "Host" } else { &me.role },
            super::field(
                "Display name",
                &format!("<input class=\\"input\\" name=\\"name\\" value=\\"{}\\" maxlength=\\"60\\">", esc(&me.name)),
                ""
            ),
            super::field(
                "Bio",
                &format!("<textarea class=\\"input\\" name=\\"bio\\" rows=\\"3\\" maxlength=\\"240\\">{}</textarea>", esc(&me.bio)),
                "Visible to everyone on this server."
            )
        ),
    };
    let title = match tab {
        "devices" => "My devices",
        "security" => "Security",
        _ => "Profile",
    };
    format!(
        "{}{}",
        topbar("history.back()", title, &me.name, ""),
        super::scroll_area("account-scroll", &body, "")
    )
}
