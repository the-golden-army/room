use crate::models::{Boot, CallDto, ConvDto, UserDto};
use crate::util::{clock, esc, human_duration};

use super::{avatar, empty_state, icon, skeleton_list, topbar};

pub fn list_column(
    boot: &Boot,
    history: &[CallDto],
    convs: &[ConvDto],
    active: Option<&CallDto>,
) -> String {
    let header = super::listcol_header(
        "Calls",
        &format!(
            "<button class=\"btn btn-brand btn-xs\" data-act=\"new-call\" type=\"button\">{} New call</button>",
            icon("plus", "h-3.5 w-3.5")
        ),
    );
    let mut items = String::new();
    if let Some(c) = active {
        items.push_str(&format!(
            "<div class=\"mx-2 mb-2 rounded-xl border p-2.5\" style=\"border-color:color-mix(in srgb, #10b981 45%, transparent);background:color-mix(in srgb, #10b981 8%, transparent)\">\
<div class=\"flex items-center gap-2\"><span class=\"relative flex h-2 w-2\"><span class=\"absolute inline-flex h-full w-full animate-ping rounded-full bg-green-400 opacity-75\"></span><span class=\"relative inline-flex h-2 w-2 rounded-full bg-green-500\"></span></span><span class=\"text-[11px] font-bold uppercase tracking-wide text-green-600 dark:text-green-400\">In call</span></div>\
<div class=\"mt-1.5 truncate text-[13px] font-semibold\">{}</div>\
<div class=\"text-[11px] ink3\">started {}</div></div>",
            esc(&c.starter_name),
            human_duration(crate::util::now_ms() - c.started_at)
        ));
    }
    items.push_str("<div class=\"px-3 pb-1 pt-2 text-[10.5px] font-bold uppercase tracking-[0.06em] ink3\">Start a call</div>");
    for c in convs.iter().take(6) {
        items.push_str(&format!(
            "<button class=\"list-item\" data-call-conv=\"{}\" type=\"button\">{}<span class=\"min-w-0 flex-1 truncate text-[13.5px] font-medium\">{}</span><span class=\"ink3\">{}</span></button>",
            c.id,
            avatar(
                &UserDto {
                    id: 0,
                    name: c.title.clone(),
                    handle: String::new(),
                    avatar: String::new(),
                    color: c.color.clone(),
                    initials: c.initials.clone(),
                    role: String::new(),
                    state: String::new(),
                    presence: String::new(),
                    bio: String::new(),
                    last_seen_at: 0,
                    online: false,
                },
                "h-9 w-9 text-xs",
                false
            ),
            esc(&c.title),
            icon("phone", "h-4 w-4")
        ));
    }
    if !history.is_empty() {
        items.push_str("<div class=\"px-3 pb-1 pt-3 text-[10.5px] font-bold uppercase tracking-[0.06em] ink3\">Recent</div>");
        for c in history.iter().take(8) {
            let dur = if c.duration_ms > 0 {
                clock(c.duration_ms)
            } else {
                "missed".into()
            };
            items.push_str(&format!(
                "<div class=\"list-item cursor-default\">{}<span class=\"min-w-0 flex-1\"><span class=\"block truncate text-[13px] font-medium\">{}</span><span class=\"block text-[10.5px] ink3\">{} · {}</span></span>{}</div>",
                icon(if c.duration_ms > 0 { "phone" } else { "phone-off" }, "h-4 w-4"),
                esc(&c.starter_name),
                human_duration(crate::util::now_ms() - c.started_at),
                dur,
                if c.status == "active" {
                    "<span class=\"chip\" style=\"background:color-mix(in srgb,#10b981 14%,transparent);color:#059669\">live</span>".to_string()
                } else {
                    String::new()
                }
            ));
        }
    }
    let list = if convs.is_empty() {
        empty_state(
            "phone",
            "No contacts yet",
            "Start a conversation first — calls are tied to a chat so the history stays in context.",
            "<a class=\"btn btn-brand\" href=\"/\">Open chat</a>",
        )
    } else {
        format!("<div class=\"flex flex-col gap-0.5 p-2\">{}</div>", items)
    };
    let _ = boot;
    format!(
        "{header}{scroll}",
        header = header,
        scroll = super::scroll_area("calls-scroll", &list, "")
    )
}

pub fn lobby(boot: &Boot, convs: &[ConvDto], kind: &str) -> String {
    let mut opts = String::new();
    for c in convs {
        opts.push_str(&format!(
            "<option value=\"{}\">{}</option>",
            c.id,
            esc(&c.title)
        ));
    }
    let body = format!(
        "<div class=\"mx-auto flex w-full max-w-lg flex-col gap-4 px-4 py-8\">\
<div class=\"panel rounded-2xl p-5 shadow-soft\">\
<div class=\"flex items-center gap-3\">\
<div class=\"grid h-12 w-12 place-items-center rounded-2xl\" style=\"background:color-mix(in srgb, var(--color-brand-600) 12%, transparent);color:var(--color-brand-600)\">{head}</div>\
<div><div class=\"text-[15px] font-bold\">Start a call</div><div class=\"text-[12px] ink3\">Direct media on this network — no external servers.</div></div></div>\
<label class=\"mt-5 block\"><span class=\"mb-1.5 block text-xs font-semibold ink2\">Conversation</span>\
<select id=\"call-conv\" class=\"input\">{opts}</select></label>\
<div class=\"mt-4\"><span class=\"mb-1.5 block text-xs font-semibold ink2\">Call type</span>\
<div class=\"grid grid-cols-2 gap-2\">\
<button class=\"btn btn-soft call-kind\" data-kind=\"audio\" type=\"button\">{mic} Audio</button>\
<button class=\"btn btn-brand call-kind\" data-kind=\"video\" type=\"button\">{cam} Video</button></div></div>\
<button id=\"join-call\" class=\"btn btn-brand mt-5 w-full\" type=\"button\">{join} Join call</button>\
<div class=\"mt-3 text-center text-[11px] ink3\">Media stays peer-to-peer on the LAN. Signalling goes through this server.</div>\
</div></div>",
        head = icon("phone", "h-6 w-6"),
        opts = opts,
        mic = icon("mic", "h-4 w-4"),
        cam = icon("video", "h-4 w-4"),
        join = icon("phone", "h-4 w-4")
    );
    let _ = boot;
    let _ = kind;
    format!(
        "{}{}",
        topbar("", "Calls", "Voice & video", ""),
        super::scroll_area("calls-main", &body, "")
    )
}

pub fn stage(
    boot: &Boot,
    call: &CallDto,
    tiles: &[Tile],
    elapsed_ms: i64,
) -> String {
    let mut grid = String::new();
    let cols = if tiles.len() <= 1 {
        "grid-cols-1"
    } else if tiles.len() <= 4 {
        "grid-cols-2"
    } else {
        "grid-cols-2 lg:grid-cols-3"
    };
    for t in tiles {
        grid.push_str(&tile_html(t));
    }
    let body = format!(
        "<div class=\"flex min-h-0 flex-1 flex-col bg-[var(--color-panel2)]\">\
<div class=\"flex items-center justify-between px-4 py-2.5\">\
<div class=\"flex items-center gap-2 text-[12px] ink2\">\
<span class=\"relative flex h-2 w-2\"><span class=\"absolute inline-flex h-full w-full animate-ping rounded-full bg-green-400 opacity-75\"></span><span class=\"relative inline-flex h-2 w-2 rounded-full bg-green-500\"></span></span>\
<span class=\"font-semibold\">{}</span>·<span id=\"call-timer\" data-start=\"{}\">{}</span></div>\
<div class=\"text-[11px] ink3\">{} participants</div></div>\
<div id=\"call-stage\" data-call=\"{call_id}\" class=\"grid {cols} min-h-0 flex-1 gap-2 overflow-y-auto p-2\">{grid}</div></div>",
        esc(&call.starter_name),
        call.started_at,
        clock(elapsed_ms),
        tiles.len(),
        call_id = call.id,
        cols = cols,
        grid = grid
    );
    let controls = format!(
        "<div class=\"flex items-center justify-center gap-2 border-t line px-3 py-3\">\
<button id=\"ctl-mic\" class=\"btn btn-soft btn-icon !h-11 !w-11 !rounded-full\" type=\"button\" title=\"Microphone\">{}</button>\
<button id=\"ctl-cam\" class=\"btn btn-soft btn-icon !h-11 !w-11 !rounded-full\" type=\"button\" title=\"Camera\">{}</button>\
<button id=\"ctl-share\" class=\"btn btn-soft btn-icon !h-11 !w-11 !rounded-full\" type=\"button\" title=\"Share screen\">{}</button>\
<button id=\"ctl-leave\" class=\"btn btn-icon !h-11 !w-11 !rounded-full\" style=\"background:#ef4444;color:#fff\" type=\"button\" title=\"Leave\">{}</button>\
</div>",
        icon("mic", "h-5 w-5"),
        icon("video", "h-5 w-5"),
        icon("share", "h-5 w-5"),
        icon("phone-off", "h-5 w-5")
    );
    let _ = boot;
    format!(
        "{}{}{}",
        topbar("history.back()", "Call", &call.starter_name, ""),
        body,
        controls
    )
}

pub struct Tile {
    pub user_id: i64,
    pub name: String,
    pub color: String,
    pub initials: String,
    pub speaking: bool,
    pub video: bool,
    pub muted: bool,
    pub local: bool,
}

fn tile_html(t: &Tile) -> String {
    let ring = if t.speaking {
        "box-shadow:0 0 0 2px #10b981".to_string()
    } else {
        String::new()
    };
    let badge = if t.video {
        String::new()
    } else {
        format!(
            "<div class=\"absolute inset-0 grid place-items-center rounded-2xl\" style=\"background:var(--color-panel3)\">{}</div>",
            avatar(
                &UserDto {
                    id: t.user_id,
                    name: t.name.clone(),
                    handle: String::new(),
                    avatar: String::new(),
                    color: t.color.clone(),
                    initials: t.initials.clone(),
                    role: String::new(),
                    state: String::new(),
                    presence: String::new(),
                    bio: String::new(),
                    last_seen_at: 0,
                    online: false,
                },
                "h-16 w-16 text-lg",
                false
            )
        )
    };
    let video_el = if t.video {
        format!("<video id=\"cam-{}\" class=\"h-full w-full rounded-2xl object-cover\" autoplay playsinline muted={}></video>", t.user_id, if t.local { "muted" } else { "" })
    } else {
        String::new()
    };
    format!(
        "<div class=\"relative min-h-[180px] overflow-hidden rounded-2xl border line bg-[var(--color-panel)]\" style=\"{ring}\">\
{video}{badge}\
<div class=\"absolute bottom-2 left-2 flex items-center gap-1.5 rounded-full px-2 py-1 text-[11px] font-semibold text-white\" style=\"background:rgb(0 0 0 / .5)\">\
<span>{name}</span>{mute}</div>\
</div>",
        ring = ring,
        video = video_el,
        badge = badge,
        name = esc(&t.name),
        mute = if t.muted {
            format!("<span class=\"text-red-300\">{}</span>", icon("mic-off", "h-3 w-3"))
        } else {
            String::new()
        }
    )
}

pub fn details_panel(call: &CallDto, users: &[UserDto]) -> String {
    let mut out = super::listcol_header("Participants", "");
    let mut body = String::new();
    for uid in &call.participants {
        if let Some(u) = users.iter().find(|u| u.id == *uid) {
            body.push_str(&format!(
                "<div class=\"navlink\">{}<span class=\"min-w-0 flex-1 truncate\">{}</span><span class=\"chip\">{}</span></div>",
                avatar(u, "h-8 w-8 text-[11px]", false),
                esc(&u.name),
                if *uid == call.started_by { "host" } else { "guest" }
            ));
        }
    }
    out.push_str(&super::scroll_area("call-details", &body, "p-2"));
    out
}

pub fn skeleton_calls() -> String {
    skeleton_list(6)
}
