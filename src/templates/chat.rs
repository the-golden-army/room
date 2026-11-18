use crate::models::{Boot, ConvDto, MsgDto, UserDto};
use crate::util::{esc, hhmm};

use super::{avatar, empty_state, icon, topbar};

pub fn conv_item(c: &ConvDto, active: bool, users: &[UserDto]) -> String {
    let _ = users;
    let unread = if c.unread > 0 {
        format!(
            "<span class=\"ml-auto inline-flex h-[18px] min-w-[18px] shrink-0 items-center justify-center rounded-full px-1.5 text-[10px] font-bold leading-none text-white\" style=\"background:{}\">{}</span>",
            if c.muted { "var(--color-ink3)" } else { "var(--color-brand-600)" },
            if c.unread > 99 { "99+".to_string() } else { c.unread.to_string() }
        )
    } else if c.muted {
        "<span class=\"ml-auto ink3\">{}</span>".to_string()
    } else {
        String::new()
    };
    let mute_icon = if c.muted {
        super::icon("bell-off", "h-3.5 w-3.5")
    } else {
        String::new()
    };
    let preview = if c.last_preview.is_empty() {
        "No messages yet".to_string()
    } else {
        let who = if c.mine_preview() {
            "You: ".to_string()
        } else if c.kind == "group" && !c.last_sender.is_empty() {
            format!("{}: ", c.last_sender)
        } else {
            String::new()
        };
        format!("{}{}", who, c.last_preview)
    };
    let active_cls = if active { " active" } else { "" };
    format!(
        "<a href=\"/chat/{id}\" class=\"conv-item{active}\" data-conv=\"{id}\" data-kind=\"{kind}\" data-unread=\"{unread_flag}\">\
{avatar}\
<span class=\"min-w-0 flex-1\"><span class=\"flex items-center gap-1.5\">\
<span class=\"truncate text-[13.5px] font-semibold\">{title}</span>{mute}\
<span class=\"ml-auto shrink-0 text-[10px] ink3\">{time}</span></span>\
<span class=\"mt-0.5 block truncate text-xs ink3\">{preview}</span></span>{unread}</a>",
        id = c.id,
        active = active_cls,
        avatar = avatar(
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
                online: users.iter().any(|u| c.members.contains(&u.id) && u.online),
            },
            "h-10 w-10 text-[13px]",
            false
        ),
        mute = mute_icon,
        time = if c.last_message_at > 0 { hhmm(c.last_message_at) } else { "".into() },
        title = esc(&c.title),
        preview = esc(&preview),
        unread = unread,
        kind = c.kind,
        unread_flag = if c.unread > 0 { "1" } else { "0" },
    )
}

impl ConvDto {
    fn mine_preview(&self) -> bool {
        self.last_sender == "You"
    }
}

pub fn list_column(
    state: &crate::state::AppState,
    boot: &Boot,
    convs: &[ConvDto],
    active: i64,
) -> String {
    let header = super::listcol_header(
        "Messages",
        &format!(
            "<button class=\"btn btn-brand btn-xs\" data-new-chat type=\"button\" title=\"New conversation\">{} New</button>",
            icon("plus", "h-3.5 w-3.5")
        ),
    );
    let search = super::search_box("conv-search", "Search people and messages…", "");
    let mut items = String::new();
    for c in convs {
        items.push_str(&conv_item(c, c.id == active, &boot.users));
    }
    let list = if convs.is_empty() {
        empty_state(
            "chat",
            "No conversations yet",
            "Start a private chat or create a group to collaborate with people on this network.",
            "<button class=\"btn btn-brand\" data-new-chat type=\"button\">Start a conversation</button>",
        )
    } else {
        format!("<div id=\"conv-list\" class=\"flex flex-col gap-0.5 p-2\">{}</div>", items)
    };
    let filters = super::filter_tabs(
        "conv-filter",
        &[
            ("all", "All", true),
            ("people", "People", false),
            ("groups", "Groups", false),
            ("unread", "Unread", false),
        ],
    );
    let _ = state;
    format!(
        "{header}{search}{filters}{scroll}",
        header = header,
        search = search,
        filters = filters,
        scroll = super::scroll_area("conv-scroll", &list, "pb-3")
    )
}

pub fn messages_html(msgs: &[MsgDto]) -> String {
    let mut out = String::new();
    let mut last: Option<&MsgDto> = None;
    let mut last_day = String::new();
    for m in msgs {
        let day = day_key(m.created_at);
        if day != last_day {
            out.push_str(&format!(
                "<div class=\"day-divider\"><span>{}</span></div>",
                day_label(m.created_at)
            ));
            last_day = day;
            last = None;
        }
        let grouped = match last {
            Some(p) => {
                p.sender_id == m.sender_id
                    && !p.deleted
                    && !m.deleted
                    && (m.created_at - p.created_at).abs() < 5 * 60_000
                    && p.reply_to_id == 0
            }
            None => false,
        };
        out.push_str(&message_html(m, grouped));
        last = Some(m);
    }
    out
}

/// Stable per-day bucket used to decide where a day divider belongs.
pub fn day_key(ts: i64) -> String {
    use chrono::{Local, TimeZone};
    match Local.timestamp_millis_opt(ts).single() {
        Some(d) => d.format("%Y-%m-%d").to_string(),
        None => String::new(),
    }
}

/// "Today", "Yesterday" or a short date, matching the reference design.
pub fn day_label(ts: i64) -> String {
    use chrono::{Local, TimeZone};
    let d = match Local.timestamp_millis_opt(ts).single() {
        Some(d) => d,
        None => return String::new(),
    };
    let today = Local::now().date_naive();
    let days = (today - d.date_naive()).num_days();
    match days {
        0 => "Today".to_string(),
        1 => "Yesterday".to_string(),
        n if n < 7 => d.format("%A").to_string(),
        _ => d.format("%d %b %Y").to_string(),
    }
}

pub fn message_html(m: &MsgDto, grouped: bool) -> String {
    if m.deleted {
        return format!(
            "<div class=\"px-4 py-[3px] text-xs italic ink3\" data-msg=\"{}\">Message deleted</div>",
            m.id
        );
    }
    let avatar_slot = if grouped {
        "<div class=\"w-9 shrink-0\"></div>".to_string()
    } else {
        format!(
            "<div class=\"w-9 shrink-0 pt-0.5\">{}</div>",
            avatar(
                &UserDto {
                    id: m.sender_id,
                    name: m.sender_name.clone(),
                    handle: String::new(),
                    avatar: m.sender_avatar.clone(),
                    color: m.sender_color.clone(),
                    initials: m.sender_initials.clone(),
                    role: String::new(),
                    state: String::new(),
                    presence: String::new(),
                    bio: String::new(),
                    last_seen_at: 0,
                    online: false,
                },
                "h-9 w-9 text-xs",
                false
            )
        )
    };
    let head = if grouped {
        String::new()
    } else {
        format!(
            "<div class=\"flex items-baseline gap-2\"><span class=\"text-[13px] font-semibold\" style=\"color:{}\">{}</span><span class=\"text-[10px] ink3\">{}</span></div>",
            m.sender_color,
            esc(&m.sender_name),
            hhmm(m.created_at)
        )
    };
    let reply = if m.reply_to_id > 0 {
        format!(
            "<div class=\"mb-1 flex items-center gap-2 rounded-lg border-l-2 px-2.5 py-1 text-[11.5px]\" style=\"border-color:{color};background:color-mix(in srgb, {color} 7%, transparent)\">\
<span class=\"font-semibold\" style=\"color:{color}\">{name}</span><span class=\"truncate ink3\">{body}</span></div>",
            color = m.sender_color,
            name = esc(&m.reply_to_name),
            body = esc(&m.reply_to_body)
        )
    } else {
        String::new()
    };
    let mut reacts = String::new();
    if !m.reactions.is_empty() {
        reacts.push_str("<div class=\"mt-1 flex flex-wrap gap-1\">");
        for r in &m.reactions {
            let mine = if r.mine { "brand" } else { "" };
            let bg = if r.mine {
                "background:color-mix(in srgb, var(--color-brand-600) 14%, transparent);border-color:color-mix(in srgb, var(--color-brand-600) 32%, transparent);color:var(--color-ink)"
            } else {
                "background:var(--color-panel2);color:var(--color-ink2)"
            };
            reacts.push_str(&format!(
                "<button class=\"chip {mine} reaction-btn\" data-message=\"{}\" data-emoji=\"{}\" style=\"{}\">{} <span class=\"opacity-70\">{}</span></button>",
                m.id,
                esc(&r.emoji),
                bg,
                esc(&r.emoji),
                r.users.len()
            ));
        }
        reacts.push_str("</div>");
    }
    let edited = if m.edited {
        "<span class=\"text-[9.5px] ink3\">(edited)</span>"
    } else {
        ""
    };
    let actions = format!(
        "<div class=\"pointer-events-none absolute -top-3 {apos} z-10 flex translate-y-1 items-center gap-0.5 rounded-xl border line panel px-1 py-0.5 opacity-0 shadow-soft transition-all group-hover:pointer-events-auto group-hover:translate-y-0 group-hover:opacity-100\">\
<button class=\"btn btn-ghost btn-icon !h-7 !w-7 !rounded-lg\" data-act=\"reply\" title=\"Reply\">{}</button>\
<button class=\"btn btn-ghost btn-icon !h-7 !w-7 !rounded-lg\" data-act=\"react\" title=\"React\">{}</button>\
{mine_actions}\
</div>",
        icon("reply", "h-3.5 w-3.5"),
        icon("smile", "h-3.5 w-3.5"),
        apos = if m.mine { "left-3" } else { "right-3" },
        mine_actions = if m.mine {
            format!(
                "<button class=\"btn btn-ghost btn-icon !h-7 !w-7 !rounded-lg\" data-act=\"edit\" title=\"Edit\">{}</button>\
<button class=\"btn btn-ghost btn-icon !h-7 !w-7 !rounded-lg hover:!text-red-500\" data-act=\"delete\" title=\"Delete\">{}</button>",
                icon("edit", "h-3.5 w-3.5"),
                icon("trash", "h-3.5 w-3.5")
            )
        } else {
            String::new()
        }
    );
    let time = hhmm(m.created_at);
    let wrapper = if grouped {
        "<div class=\"w-9 shrink-0\"></div>"
    } else {
        avatar_slot.as_str()
    };
    let own_class = if m.mine { " own" } else { "" };
    let head_html = if m.mine { "" } else { head.as_str() };
    format!(
        "<div class=\"group relative bubble-row{own_class}\" data-msg=\"{id}\" data-sender=\"{sender}\" data-grouped=\"{grouped}\">\
{wrapper}\
<div class=\"flex min-w-0 flex-1 flex-col {align}\">\
{head_html}\
<div class=\"bubble\">{reply}<div class=\"msg-body\">{body}</div><div class=\"bubble-meta\"><span>{time}</span>{edited}</div></div>\
{reacts}\
</div>\
{actions}\
</div>",
        id = m.id,
        sender = m.sender_id,
        grouped = if grouped { "1" } else { "0" },
        own_class = own_class,
        align = if m.mine { "items-end" } else { "items-start" },
        wrapper = wrapper,
        head_html = head_html,
        reply = reply,
        body = crate::templates::render_body(&m.body),
        time = time,
        edited = edited,
        reacts = reacts,
        actions = actions,
    )
}

pub fn main_column(
    boot: &Boot,
    conv: Option<&ConvDto>,
    msgs: &[MsgDto],
    users: &[UserDto],
) -> String {
    match conv {
        None => {
            let main = empty_state(
                "chat",
                "Pick a conversation",
                "Choose a chat on the left, or start a new one. Messages are stored locally on this server.",
                "<button class=\"btn btn-brand\" data-new-chat type=\"button\">New conversation</button>",
            );
            format!(
                "{}{}",
                topbar(
                    "",
                    "Messages",
                    &format!("{} online", boot.users.iter().filter(|u| u.online).count()),
                    &format!(
                        "<button class=\"btn btn-ghost btn-icon\" data-new-chat type=\"button\" title=\"New conversation\">{}</button>",
                        icon("edit", "h-4 w-4")
                    ),
                ),
                super::scroll_area("messages", &main, "")
            )
        }
        Some(c) => {
            let online = users
                .iter()
                .filter(|u| c.members.contains(&u.id) && u.online)
                .count();
            let sub = if c.kind == "group" {
                format!("{} members · {} online", c.members.len(), online)
            } else {
                let other = users.iter().find(|u| c.members.contains(&u.id) && u.id != boot.me.id);
                match other {
                    Some(u) if u.online => "Online".to_string(),
                    Some(_) => "Last seen recently".to_string(),
                    None => String::new(),
                }
            };
            let mut actions = String::new();
            if c.kind == "group" {
                actions.push_str(&format!(
                    "<button class=\"btn btn-ghost btn-icon\" data-act=\"add-member\" title=\"Add member\">{}</button>",
                    icon("plus", "h-4 w-4")
                ));
            }
            actions.push_str(&format!(
                "<a class=\"btn btn-ghost btn-icon\" href=\"/files\" title=\"Shared files\">{}</a>",
                icon("folder", "h-4 w-4")
            ));
            let history = if msgs.is_empty() {
                empty_state(
                    "inbox",
                    "No messages yet",
                    &format!("Say hi to {} — this conversation stays on this server.", c.title),
                    "",
                )
            } else {
                messages_html(msgs)
            };
            let composer = format!(
                "<form id=\"composer\" class=\"px-3 pb-3 pt-1 md:px-4\" data-conv=\"{id}\">\
<div class=\"mb-1.5 flex h-4 items-center justify-center gap-1.5 text-[10.5px] ink3\">\
<span class=\"inline-flex items-center gap-1\">{lock} Messages stay on this server</span>\
<span id=\"typing\" class=\"font-semibold\" style=\"color:var(--color-brand-600)\"></span></div>\
<div class=\"panel flex items-end gap-1 rounded-[22px] py-1 pl-1.5 pr-1.5 shadow-soft focus-within:border-[var(--color-brand-500)]\">\
<button type=\"button\" id=\"attach-btn\" class=\"btn btn-ghost btn-icon !h-9 !w-9 !rounded-full\" title=\"Attach a file\">{clip}</button>\
<textarea id=\"composer-input\" rows=\"1\" class=\"max-h-40 flex-1 resize-none self-center bg-transparent px-1.5 py-2 text-[13.5px] outline-none\" placeholder=\"Message {title}…\"></textarea>\
<button type=\"button\" id=\"emoji-btn\" class=\"btn btn-ghost btn-icon !h-9 !w-9 !rounded-full\" title=\"Emoji\">{smile}</button>\
<button type=\"submit\" id=\"send-btn\" class=\"btn btn-brand btn-icon !h-9 !w-9 !rounded-full\" title=\"Send\">{send}</button>\
</div>\
<input type=\"file\" id=\"file-input\" class=\"hidden\" multiple>\
</form>",
                id = c.id,
                lock = icon("lock", "h-3 w-3"),
                clip = icon("plus", "h-[18px] w-[18px]"),
                title = esc(&c.title),
                smile = icon("smile", "h-[18px] w-[18px]"),
                send = icon("send", "h-[18px] w-[18px]"),
            );
            format!(
                "{}{}{}",
                topbar(
                    "history.back()",
                    &c.title,
                    &sub,
                    &actions,
                ),
                super::scroll_area(
                    "messages",
                    &history,
                    "px-1 py-3 space-y-[3px] md:px-2"
                ),
                composer
            )
        }
    }
}

pub fn details_panel(boot: &Boot, conv: &ConvDto) -> String {
    let mut out = String::new();
    out.push_str(&super::listcol_header(
        "Details",
        &format!(
            "<button class=\"btn btn-ghost btn-icon\" data-act=\"close-panel\" title=\"Close\">{}</button>",
            icon("x", "h-4 w-4")
        ),
    ));
    let mut body = String::new();
    if conv.kind == "group" {
        body.push_str(&format!(
            "<div class=\"flex items-center gap-3 px-4 py-4\">{}<div class=\"min-w-0\"><div class=\"truncate text-sm font-bold\">{}</div><div class=\"text-xs ink3\">Group · {} members</div></div></div>",
            avatar(
                &UserDto {
                    id: 0,
                    name: conv.title.clone(),
                    handle: String::new(),
                    avatar: String::new(),
                    color: conv.color.clone(),
                    initials: conv.initials.clone(),
                    role: String::new(),
                    state: String::new(),
                    presence: String::new(),
                    bio: String::new(),
                    last_seen_at: 0,
                    online: false,
                },
                "h-12 w-12 text-base",
                false
            ),
            esc(&conv.title),
            conv.members.len()
        ));
        if !conv.topic.is_empty() {
            body.push_str(&format!(
                "<div class=\"px-4 pb-3 text-[12.5px] ink2\">{}</div>",
                esc(&conv.topic)
            ));
        }
        body.push_str("<div class=\"px-4 pb-2 text-[11px] font-bold uppercase tracking-[0.06em] ink3\">Members</div>");
        for uid in &conv.members {
            if let Some(u) = boot.users.iter().find(|u| u.id == *uid) {
                body.push_str(&member_row(u));
            }
        }
    } else {
        let other = boot
            .users
            .iter()
            .find(|u| conv.members.contains(&u.id) && u.id != boot.me.id)
            .or_else(|| boot.users.iter().find(|u| u.id == boot.me.id));
        if let Some(u) = other {
            body.push_str(&format!(
                "<div class=\"flex flex-col items-center gap-2 px-4 py-6 text-center\">{}<div><div class=\"text-base font-bold\">{}</div><div class=\"text-xs ink3\">@{}</div></div><div class=\"text-xs ink2\">{}</div></div>",
                avatar(u, "h-20 w-20 text-2xl", true),
                esc(&u.name),
                esc(&u.handle),
                esc(&u.bio)
            ));
            body.push_str(&format!(
                "<div class=\"grid grid-cols-2 gap-2 px-4 pb-4\"><div class=\"tile p-2.5 text-center\"><div class=\"text-sm font-bold\">{}</div><div class=\"text-[10px] ink3\">Presence</div></div><div class=\"tile p-2.5 text-center\"><div class=\"text-sm font-bold\">{}</div><div class=\"text-[10px] ink3\">Role</div></div><div class=\"tile p-2.5 text-center\"><div class=\"text-sm font-bold\">{}</div><div class=\"text-[10px] ink3\">Last seen</div></div></div>",
                esc(u.presence.as_str()),
                esc(if u.role == "owner" { "Host" } else { &u.role }),
                if u.last_seen_at > 0 { super::rel_ago(u.last_seen_at) } else { "—".into() }
            ));
        }
    }
    let mut files = String::new();
    files.push_str("<div class=\"px-4 pb-2 pt-2 text-[11px] font-bold uppercase tracking-[0.06em] ink3\">Actions</div>");
    files.push_str(&format!(
        "<div class=\"flex flex-col gap-1 px-3 pb-4\">\
<button class=\"navlink\" data-act=\"mute-conv\" type=\"button\">{}<span>Mute notifications</span><span class=\"ml-auto chip\" data-muted=\"{}\">{}</span></button>\
<button class=\"navlink\" data-act=\"mark-read\" type=\"button\">{}<span>Mark as read</span></button>\
<a class=\"navlink\" href=\"/files?conv={}\">{}<span>Shared files</span>{}</a>\
</div>",
        icon("bell", "h-4 w-4"),
        if conv.muted { "1" } else { "0" },
        if conv.muted { "Muted" } else { "On" },
        icon("check", "h-4 w-4"),
        conv.id,
        icon("folder", "h-4 w-4"),
        icon("chev-right", "h-3.5 w-3.5 ml-auto")
    ));
    out.push_str(&super::scroll_area("details-scroll", &format!("{}{}", body, files), ""));
    out
}

fn member_row(u: &UserDto) -> String {
    format!(
        "<div class=\"navlink\">{}<span class=\"min-w-0 flex-1 truncate\">{}</span><span class=\"text-[11px] ink3\">{}</span></div>",
        avatar(u, "h-8 w-8 text-[11px]", true),
        esc(&u.name),
        if u.online { "online" } else { "offline" }
    )
}
