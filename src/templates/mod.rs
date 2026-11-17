use crate::models::{Boot, UserDto};
use crate::util::{esc, human_duration, js_str};

pub mod account;
pub mod admin;
pub mod calls;
pub mod chat;
pub mod files;
pub mod gate;

pub const CSS_PATH: &str = "/assets/app.css";
pub const JS_PATH: &str = "/assets/app.js";

// ------------------------------------------------------------------ icons

/// Inline stroke icons (Lucide geometry). `cls` overrides the svg class list.
pub fn icon(name: &str, cls: &str) -> String {
    let paths = match name {
        "chat" => "<path d=\"M21 11.5a8.38 8.38 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.38 8.38 0 0 1-3.8-.9L3 21l1.9-5.7a8.38 8.38 0 0 1-.9-3.8 8.5 8.5 0 0 1 4.7-7.6 8.38 8.38 0 0 1 3.8-.9h.5a8.48 8.48 0 0 1 8 8v.5z\"/>",
        "phone" => "<path d=\"M22 16.92v3a2 2 0 0 1-2.18 2 19.79 19.79 0 0 1-8.63-3.07 19.5 19.5 0 0 1-6-6 19.79 19.79 0 0 1-3.07-8.67A2 2 0 0 1 4.11 2h3a2 2 0 0 1 2 1.72 12.84 12.84 0 0 0 .7 2.81 2 2 0 0 1-.45 2.11L8.09 9.91a16 16 0 0 0 6 6l1.27-1.27a2 2 0 0 1 2.11-.45 12.84 12.84 0 0 0 2.81.7A2 2 0 0 1 22 16.92z\"/>",
        "folder" => "<path d=\"M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z\"/>",
        "shield" => "<path d=\"M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z\"/>",
        "search" => "<circle cx=\"11\" cy=\"11\" r=\"8\"/><path d=\"m21 21-4.3-4.3\"/>",
        "plus" => "<path d=\"M5 12h14\"/><path d=\"M12 5v14\"/>",
        "send" => "<path d=\"m22 2-7 20-4-9-9-4Z\"/><path d=\"M22 2 11 13\"/>",
        "paperclip" => "<path d=\"m21.44 11.05-9.19 9.19a6 6 0 0 1-8.49-8.49l8.57-8.57A4 4 0 1 1 18 8.84l-8.59 8.57a2 2 0 0 1-2.83-2.83l8.49-8.48\"/>",
        "smile" => "<circle cx=\"12\" cy=\"12\" r=\"10\"/><path d=\"M8 14s1.5 2 4 2 4-2 4-2\"/><line x1=\"9\" x2=\"9.01\" y1=\"9\" y2=\"9\"/><line x1=\"15\" x2=\"15.01\" y1=\"9\" y2=\"9\"/>",
        "reply" => "<path d=\"m9 17-5-5 5-5\"/><path d=\"M4 12h9a5 5 0 0 1 5 5v1\"/>",
        "trash" => "<path d=\"M3 6h18\"/><path d=\"M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6\"/><path d=\"M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2\"/><line x1=\"10\" x2=\"10\" y1=\"11\" y2=\"17\"/><line x1=\"14\" x2=\"14\" y1=\"11\" y2=\"17\"/>",
        "edit" => "<path d=\"M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z\"/><path d=\"m15 5 4 4\"/>",
        "more" => "<circle cx=\"12\" cy=\"12\" r=\"1\"/><circle cx=\"19\" cy=\"12\" r=\"1\"/><circle cx=\"5\" cy=\"12\" r=\"1\"/>",
        "sun" => "<circle cx=\"12\" cy=\"12\" r=\"4\"/><path d=\"M12 2v2\"/><path d=\"M12 20v2\"/><path d=\"m4.93 4.93 1.41 1.41\"/><path d=\"m17.66 17.66 1.41 1.41\"/><path d=\"M2 12h2\"/><path d=\"M20 12h2\"/><path d=\"m6.34 17.66-1.41 1.41\"/><path d=\"m19.07 4.93-1.41 1.41\"/>",
        "moon" => "<path d=\"M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z\"/>",
        "x" => "<path d=\"M18 6 6 18\"/><path d=\"m6 6 12 12\"/>",
        "check" => "<path d=\"M20 6 9 17l-5-5\"/>",
        "download" => "<path d=\"M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4\"/><path d=\"m7 10 5 5 5-5\"/><path d=\"M12 15V3\"/>",
        "upload" => "<path d=\"M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4\"/><path d=\"m7 8 5-5 5 5\"/><path d=\"M12 3v12\"/>",
        "folder-plus" => "<path d=\"M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z\"/><path d=\"M12 10v6\"/><path d=\"M9 13h6\"/>",
        "file" => "<path d=\"M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z\"/><path d=\"M14 2v4a2 2 0 0 0 2 2h4\"/>",
        "file-text" => "<path d=\"M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z\"/><path d=\"M14 2v4a2 2 0 0 0 2 2h4\"/><path d=\"M10 9H8\"/><path d=\"M16 13H8\"/><path d=\"M16 17H8\"/>",
        "image" => "<rect width=\"18\" height=\"18\" x=\"3\" y=\"3\" rx=\"2\"/><circle cx=\"9\" cy=\"9\" r=\"2\"/><path d=\"m21 15-3.086-3.086a2 2 0 0 0-2.828 0L6 21\"/>",
        "video" => "<path d=\"m22 8-6 4 6 4V8Z\"/><rect width=\"14\" height=\"12\" x=\"2\" y=\"6\" rx=\"2\"/>",
        "video-off" => "<path d=\"M10.5 6H8a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2v-2.5\"/><path d=\"M22 8l-4 2.5V8\"/><path d=\"m2 2 20 20\"/>",
        "mic" => "<path d=\"M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z\"/><path d=\"M19 10v2a7 7 0 0 1-14 0v-2\"/><line x1=\"12\" x2=\"12\" y1=\"19\" y2=\"22\"/>",
        "mic-off" => "<path d=\"M9 9v3a3 3 0 0 0 5.12 2.12\"/><path d=\"M15 9.34V5a3 3 0 0 0-5.94-.6\"/><path d=\"M19 10v2a7 7 0 0 1-.11 1.23\"/><path d=\"M5 10v2a7 7 0 0 0 12 5\"/><path d=\"M12 19v3\"/><path d=\"m2 2 20 20\"/>",
        "share" => "<path d=\"M13 3H4a2 2 0 0 0-2 2v10a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-3\"/><path d=\"M8 21h8\"/><path d=\"M12 17v4\"/><path d=\"m17 8 5-5\"/><path d=\"M12 3h5v5\"/>",
        "phone-off" => "<path d=\"M10.68 13.31a16 16 0 0 0 3.41 2.6l1.27-1.27a2 2 0 0 1 2.11-.45 12.84 12.84 0 0 0 2.81.7 2 2 0 0 1 1.72 2v3a2 2 0 0 1-2.18 2 19.79 19.79 0 0 1-8.63-3.07 19.42 19.42 0 0 1-3.33-2.67m-2.67-3.34a19.79 19.79 0 0 1-2.05-8.36A2 2 0 0 1 4.11 2h3a2 2 0 0 1 2 1.72 12.84 12.84 0 0 0 .7 2.81 2 2 0 0 1-.45 2.11L8.09 9.91\"/><path d=\"m2 2 20 20\"/>",
        "users" => "<path d=\"M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2\"/><circle cx=\"9\" cy=\"7\" r=\"4\"/><path d=\"M22 21v-2a4 4 0 0 0-3-3.87\"/><path d=\"M16 3.13a4 4 0 0 1 0 7.75\"/>",
        "clock" => "<circle cx=\"12\" cy=\"12\" r=\"10\"/><path d=\"M12 6v6l4 2\"/>",
        "chev-left" => "<path d=\"m15 18-6-6 6-6\"/>",
        "chev-right" => "<path d=\"m9 18 6-6-6-6\"/>",
        "chev-down" => "<path d=\"m6 9 6 6 6-6\"/>",
        "bell" => "<path d=\"M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9\"/><path d=\"M10.3 21a1.94 1.94 0 0 0 3.4 0\"/>",
        "bell-off" => "<path d=\"M8.7 3A6 6 0 0 1 18 8a21.3 21.3 0 0 0 .6 5\"/><path d=\"M17 17H3s3-2 3-9a4.67 4.67 0 0 1 .3-1.7\"/><path d=\"M10.3 21a1.94 1.94 0 0 0 3.4 0\"/><path d=\"m2 2 20 20\"/>",
        "database" => "<ellipse cx=\"12\" cy=\"5\" rx=\"9\" ry=\"3\"/><path d=\"M3 5v14a9 3 0 0 0 18 0V5\"/><path d=\"M3 12a9 3 0 0 0 18 0\"/>",
        "activity" => "<path d=\"M22 12h-4l-3 9L9 3l-3 9H2\"/>",
        "refresh" => "<path d=\"M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8\"/><path d=\"M21 3v5h-5\"/><path d=\"M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16\"/><path d=\"M8 16H3v5\"/>",
        "archive" => "<rect width=\"20\" height=\"5\" x=\"2\" y=\"3\" rx=\"1\"/><path d=\"M4 8v11a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8\"/><path d=\"M10 12h4\"/>",
        "info" => "<circle cx=\"12\" cy=\"12\" r=\"10\"/><path d=\"M12 16v-4\"/><path d=\"M12 8h.01\"/>",
        "warning" => "<path d=\"m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z\"/><path d=\"M12 9v4\"/><path d=\"M12 17h.01\"/>",
        "copy" => "<rect width=\"14\" height=\"14\" x=\"8\" y=\"8\" rx=\"2\"/><path d=\"M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2\"/>",
        "grid" => "<rect width=\"7\" height=\"7\" x=\"3\" y=\"3\" rx=\"1\"/><rect width=\"7\" height=\"7\" x=\"14\" y=\"3\" rx=\"1\"/><rect width=\"7\" height=\"7\" x=\"14\" y=\"14\" rx=\"1\"/><rect width=\"7\" height=\"7\" x=\"3\" y=\"14\" rx=\"1\"/>",
        "list" => "<path d=\"M8 6h13\"/><path d=\"M8 12h13\"/><path d=\"M8 18h13\"/><path d=\"M3 6h.01\"/><path d=\"M3 12h.01\"/><path d=\"M3 18h.01\"/>",
        "sort" => "<path d=\"m21 16-4 4-4-4\"/><path d=\"M17 20V4\"/><path d=\"m3 8 4-4 4 4\"/><path d=\"M7 4v16\"/>",
        "drive" => "<path d=\"M22 12H2\"/><path d=\"M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z\"/><path d=\"M6 16h.01\"/><path d=\"M10 16h.01\"/>",
        "wifi" => "<path d=\"M5 13a10 10 0 0 1 14 0\"/><path d=\"M8.5 16.5a5 5 0 0 1 7 0\"/><path d=\"M2 8.82a15 15 0 0 1 20 0\"/><path d=\"M12 20h.01\"/>",
        "key" => "<path d=\"m21 2-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0 3 3L22 7l-3-3m-3.5 3.5L19 4\"/>",
        "monitor" => "<rect width=\"20\" height=\"14\" x=\"2\" y=\"3\" rx=\"2\"/><path d=\"M8 21h8\"/><path d=\"M12 17v4\"/>",
        "smartphone" => "<rect width=\"14\" height=\"20\" x=\"5\" y=\"2\" rx=\"2\"/><path d=\"M12 18h.01\"/>",
        "user" => "<path d=\"M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2\"/><circle cx=\"12\" cy=\"7\" r=\"4\"/>",
        "lock" => "<rect width=\"18\" height=\"11\" x=\"3\" y=\"11\" rx=\"2\"/><path d=\"M7 11V7a5 5 0 0 1 10 0v4\"/>",
        "logout" => "<path d=\"M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4\"/><path d=\"m16 17 5-5-5-5\"/><path d=\"M21 12H9\"/>",
        "settings" => "<path d=\"M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z\"/><circle cx=\"12\" cy=\"12\" r=\"3\"/>",
        "eye" => "<path d=\"M2 12s3-7 10-7 10 7 10 7-3 7-10 7-10-7-10-7Z\"/><circle cx=\"12\" cy=\"12\" r=\"3\"/>",
        "link" => "<path d=\"M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71\"/><path d=\"M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71\"/>",
        "pin" => "<path d=\"M12 17v5\"/><path d=\"M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V6h1a2 2 0 0 0 0-4H8a2 2 0 0 0 0 4h1z\"/>",
        "restore" => "<path d=\"M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8\"/><path d=\"M3 3v5h5\"/>",
        "power" => "<path d=\"M12 2v10\"/><path d=\"M18.4 6.6a9 9 0 1 1-12.77.04\"/>",
        "server" => "<rect width=\"20\" height=\"8\" x=\"2\" y=\"2\" rx=\"2\"/><rect width=\"20\" height=\"8\" x=\"2\" y=\"14\" rx=\"2\"/><path d=\"M6 6h.01\"/><path d=\"M6 18h.01\"/>",
        "globe" => "<circle cx=\"12\" cy=\"12\" r=\"10\"/><path d=\"M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20\"/><path d=\"M2 12h20\"/>",
        "music" => "<path d=\"M9 18V5l12-2v13\"/><circle cx=\"6\" cy=\"18\" r=\"3\"/><circle cx=\"18\" cy=\"16\" r=\"3\"/>",
        "code" => "<path d=\"m16 18 6-6-6-6\"/><path d=\"m8 6-6 6 6 6\"/>",
        "inbox" => "<path d=\"M22 12h-6l-2 3h-4l-2-3H2\"/><path d=\"M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z\"/>",
        "hash" => "<path d=\"M4 9h16\"/><path d=\"M4 15h16\"/><path d=\"M10 3 8 21\"/><path d=\"M16 3l-2 18\"/>",
        "at" => "<circle cx=\"12\" cy=\"12\" r=\"4\"/><path d=\"M16 8v5a3 3 0 0 0 6 0v-1a10 10 0 1 0-4 8\"/>",
        "camera" => "<path d=\"M14.5 4h-5L7 7H4a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2h-3l-2.5-3z\"/><circle cx=\"12\" cy=\"13\" r=\"3\"/>",
        "dot" => "<circle cx=\"12\" cy=\"12\" r=\"5\" fill=\"currentColor\" stroke=\"none\"/>",
        "screen" => "<rect width=\"20\" height=\"14\" x=\"2\" y=\"3\" rx=\"2\"/><path d=\"M8 21h8\"/><path d=\"M12 17v4\"/>",
        _ => "<circle cx=\"12\" cy=\"12\" r=\"9\"/>",
    };
    format!(
        "<svg class=\"{}\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" stroke-linecap=\"round\" stroke-linejoin=\"round\" aria-hidden=\"true\">{}</svg>",
        cls, paths
    )
}

/// Small square glyph used as the ROOM mark.
pub fn logo_mark(cls: &str) -> String {
    format!(
        "<svg class=\"{}\" viewBox=\"0 0 32 32\" fill=\"none\" aria-hidden=\"true\">\
<rect width=\"32\" height=\"32\" rx=\"9\" fill=\"url(#roomg)\"/>\
<defs><linearGradient id=\"roomg\" x1=\"0\" y1=\"0\" x2=\"32\" y2=\"32\" gradientUnits=\"userSpaceOnUse\">\
<stop stop-color=\"#6d4dff\"/><stop offset=\"1\" stop-color=\"#4a2ae0\"/></linearGradient></defs>\
<path d=\"M9 22.5v-9.2a3.3 3.3 0 0 1 3.3-3.3h7.4a3.3 3.3 0 0 1 3.3 3.3v4.6a3.3 3.3 0 0 1-3.3 3.3h-6.4l-4.3 3.6v-2.3z\" fill=\"#fff\"/>\
<circle cx=\"13.4\" cy=\"14.4\" r=\"1.15\" fill=\"#5333f4\"/>\
<circle cx=\"16.5\" cy=\"14.4\" r=\"1.15\" fill=\"#5333f4\"/>\
<circle cx=\"19.6\" cy=\"14.4\" r=\"1.15\" fill=\"#f6960c\"/>\
</svg>",
        cls
    )
}

// ------------------------------------------------------------------ fragments

pub fn avatar(u: &UserDto, cls: &str, presence: bool) -> String {
    let dot = if presence {
        let c = match u.presence.as_str() {
            "online" => "on",
            "away" => "away",
            _ => "",
        };
        format!("<span class=\"presence {c}\"></span>")
    } else {
        String::new()
    };
    let inner = if u.avatar.is_empty() {
        esc(&u.initials)
    } else {
        esc(&u.avatar)
    };
    format!(
        "<span class=\"relative inline-block shrink-0 {cls}\">\
<span class=\"grid h-full w-full place-items-center rounded-full font-semibold text-white\" style=\"background:{bg};font-size:.72em\">{inner}</span>{dot}</span>",
        bg = u.color,
        dot = dot
    )
}

pub fn presence_dot(p: &str) -> String {
    let c = match p {
        "online" => "on",
        "away" => "away",
        _ => "",
    };
    format!("<span class=\"presence {c}\"></span>")
}

pub fn skel(w: &str, h: &str, r: &str) -> String {
    format!("<div class=\"skel\" style=\"width:{w};height:{h};border-radius:{r}\"></div>")
}

pub fn skeleton_list(n: usize) -> String {
    let mut out = String::new();
    for _ in 0..n {
        out.push_str(
            "<div class=\"flex items-center gap-3 px-3 py-2.5\">\
<div class=\"skel\" style=\"width:38px;height:38px;border-radius:999px\"></div>\
<div class=\"flex-1 space-y-2\">\
<div class=\"skel\" style=\"width:52%;height:12px\"></div>\
<div class=\"skel\" style=\"width:78%;height:10px\"></div>\
</div></div>",
        );
    }
    out
}

pub fn skeleton_msgs(n: usize) -> String {
    let mut out = String::new();
    for i in 0..n {
        let (w1, w2) = if i % 3 == 0 {
            ("38%", "26%")
        } else if i % 3 == 1 {
            ("62%", "30%")
        } else {
            ("48%", "22%")
        };
        out.push_str(&format!(
            "<div class=\"flex gap-3\"><div class=\"skel\" style=\"width:36px;height:36px;border-radius:999px\"></div>\
<div class=\"flex-1 space-y-2\"><div class=\"skel\" style=\"width:{w1};height:11px\"></div>\
<div class=\"skel\" style=\"width:{w2};height:11px\"></div></div></div>"
        ));
    }
    out
}

pub fn skeleton_grid(n: usize) -> String {
    let mut out = String::new();
    for _ in 0..n {
        out.push_str(
            "<div class=\"tile p-3 space-y-3\"><div class=\"skel\" style=\"width:100%;height:96px;border-radius:10px\"></div>\
<div class=\"skel\" style=\"width:64%;height:11px\"></div><div class=\"skel\" style=\"width:38%;height:9px\"></div></div>",
        );
    }
    out
}

pub fn skeleton_rows(n: usize) -> String {
    let mut out = String::new();
    for _ in 0..n {
        out.push_str(
            "<div class=\"flex items-center gap-3 px-3 py-2.5\">\
<div class=\"skel\" style=\"width:22px;height:22px;border-radius:6px\"></div>\
<div class=\"skel\" style=\"width:34%;height:12px\"></div>\
<div class=\"skel ml-auto\" style=\"width:60px;height:10px\"></div>\
<div class=\"skel\" style=\"width:70px;height:10px\"></div></div>",
        );
    }
    out
}

pub fn empty_state(icon_name: &str, title: &str, body: &str, action: &str) -> String {
    format!(
        "<div class=\"flex flex-col items-center justify-center gap-2 px-6 py-14 text-center\">\
<div class=\"grid h-14 w-14 place-items-center rounded-2xl panel2 ink3\">{}</div>\
<div class=\"text-sm font-semibold ink\">{}</div>\
<div class=\"max-w-xs text-[13px] leading-relaxed ink3\">{}</div>\
<div class=\"mt-2\">{}</div></div>",
        icon(icon_name, "h-6 w-6"),
        esc(title),
        esc(body),
        action
    )
}

pub fn rel_time(ms: i64) -> String {
    if ms <= 0 {
        return "—".into();
    }
    human_duration(crate::util::now_ms() - ms)
}

pub fn listcol_header(title: &str, action: &str) -> String {
    format!(
        "<div class=\"flex items-center gap-2 border-b line px-4 py-3\">\
<h2 class=\"flex-1 truncate text-[13px] font-bold uppercase tracking-[0.06em] ink2\">{}</h2>{}</div>",
        esc(title),
        action
    )
}

pub fn search_box(id: &str, placeholder: &str, value: &str) -> String {
    format!(
        "<div class=\"px-3 pb-2 pt-1\"><div class=\"relative\">\
<span class=\"pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 ink3\">{}</span>\
<input id=\"{id}\" type=\"search\" class=\"input\" style=\"padding-left:2.1rem\" placeholder=\"{}\" value=\"{}\" autocomplete=\"off\">\
</div></div>",
        icon("search", "h-4 w-4"),
        esc(placeholder),
        esc(value)
    )
}

// ------------------------------------------------------------------ shell

struct NavItem {
    href: &'static str,
    icon: &'static str,
    label: &'static str,
    key: &'static str,
    admin_only: bool,
}

const NAV: &[NavItem] = &[
    NavItem { href: "/", icon: "chat", label: "Chat", key: "chat", admin_only: false },
    NavItem { href: "/calls", icon: "phone", label: "Calls", key: "calls", admin_only: false },
    NavItem { href: "/files", icon: "folder", label: "Files", key: "files", admin_only: false },
    NavItem { href: "/admin", icon: "shield", label: "Admin", key: "admin", admin_only: true },
];

fn nav_buttons(boot: &Boot, section: &str) -> String {
    let mut out = String::new();
    for n in NAV {
        if n.admin_only && !boot.is_admin {
            continue;
        }
        let active = if section == n.key { " active" } else { "" };
        let badge = if n.key == "admin" && boot.pending_requests > 0 {
            format!("<span class=\"absolute right-1 top-1 grid h-4 min-w-4 place-items-center rounded-full px-1 text-[9px] font-bold text-white\" style=\"background:#ef4444\">{}</span>", boot.pending_requests)
        } else {
            String::new()
        };
        out.push_str(&format!(
            "<a href=\"{href}\" class=\"rail-btn{active}\" data-nav=\"{key}\" title=\"{label}\">\
<span class=\"relative\">{icon}{badge}</span><span class=\"rail-label\">{label}</span></a>",
            href = n.href,
            active = active,
            key = n.key,
            label = n.label,
            icon = icon(n.icon, "h-[21px] w-[21px]"),
            badge = badge
        ));
    }
    out
}

fn mob_nav(boot: &Boot, section: &str) -> String {
    let mut out = String::new();
    for n in NAV {
        if n.admin_only && !boot.is_admin {
            continue;
        }
        let active = if section == n.key { " active" } else { "" };
        out.push_str(&format!(
            "<a href=\"{href}\" class=\"rail-btn{active} !gap-1\" data-nav=\"{key}\">{icon}<span class=\"rail-label\">{label}</span></a>",
            href = n.href,
            active = active,
            key = n.key,
            icon = icon(n.icon, "h-[21px] w-[21px]"),
            label = n.label
        ));
    }
    out
}

/// Full HTML document. `listcol`, `main`, `side` are pre-rendered fragments.
pub fn page(
    boot: &Boot,
    section: &str,
    listcol: &str,
    main: &str,
    side: &str,
    doc_title: &str,
) -> String {
    let boot_json = serde_json::to_string(boot).unwrap_or_else(|_| "{}".into());
    let theme = boot
        .settings
        .get("theme")
        .cloned()
        .unwrap_or_else(|| "light".into());
    let dark = theme == "dark" || theme == "system";
    let me = &boot.me;

    let side_html = if side.is_empty() {
        String::new()
    } else {
        format!(
            "<aside id=\"sidecol\" class=\"w-[320px] shrink-0 border-l line panel hidden xl:flex flex-col min-h-0\">{}</aside>",
            side
        )
    };

    format!(
        r#"<!doctype html>
<html lang="en" class="{darkcls}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
<meta name="color-scheme" content="light dark">
<title>{title} · {name}</title>
<link rel="icon" href="/favicon.svg" type="image/svg+xml">
<link rel="stylesheet" href="{css}">
<script>window.__BOOT__ = {boot};</script>
<script>window.__THEME__ = {theme};</script>
</head>
<body class="antialiased">
<div id="app" class="app-shell flex h-dvh w-full overflow-hidden">
  <nav id="rail" class="w-[76px] shrink-0 flex-col items-center gap-1 border-r line panel px-2 py-2.5 hidden md:flex">
    <a href="/" class="mb-1.5 block" title="{name} server">{logo}</a>
    <div id="rail-nav" class="flex w-full flex-col gap-0.5">{nav}</div>
    <div class="mt-auto flex w-full flex-col items-center gap-0.5 pt-2">
      <button id="theme-btn" class="rail-btn" type="button" title="Toggle theme" aria-label="Toggle theme">{themeicon}<span class="rail-label">Theme</span></button>
      <a href="/account" class="relative mt-1.5 block" id="me-avatar" title="{me}">{avatar}</a>
      <a href="/gate" class="rail-btn hidden" data-nav="gate"></a>
    </div>
  </nav>
  <div id="listcol" class="flex min-h-0 w-full flex-1 flex-col border-r line panel md:w-[300px] md:flex-none">{listcol}</div>
  <main id="maincol" class="flex min-h-0 min-w-0 flex-1 flex-col">{main}</main>
  {side}
</div>
<nav id="mobnav" class="fixed inset-x-0 bottom-0 z-40 flex items-stretch justify-around border-t line panel px-1 py-1 md:hidden">{mobnav}</nav>
<div id="toasts" class="pointer-events-none fixed bottom-4 right-4 z-[90] flex flex-col items-end gap-2"></div>
<div id="modal-root"></div>
<div id="overlay-root"></div>
<script defer src="{js}"></script>
</div>
</body>
</html>"#,
        darkcls = if dark { "dark" } else { "" },
        title = esc(doc_title),
        name = esc(&boot.server.name),
        css = CSS_PATH,
        boot = js_str(&boot_json),
        theme = js_str(&theme),
        logo = logo_mark("h-11 w-11 drop-shadow-sm"),
        nav = nav_buttons(boot, section),
        themeicon = icon(if dark { "sun" } else { "moon" }, "h-[21px] w-[21px]"),
        me = esc(&me.name),
        avatar = avatar(me, "h-10 w-10 text-sm", true),
        listcol = listcol,
        side = side_html,
        mobnav = mob_nav(boot, section),
        js = JS_PATH,
    )
}

/// Top bar of the main column.
pub fn topbar(back: &str, title: &str, subtitle: &str, actions: &str) -> String {
    let back_btn = if back.is_empty() {
        String::new()
    } else {
        format!(
            "<button id=\"back-btn\" class=\"btn btn-ghost btn-icon md:hidden\" type=\"button\" aria-label=\"Back\">{}</button>",
            icon("chev-left", "h-5 w-5")
        )
    };
    let sub = if subtitle.is_empty() {
        String::new()
    } else {
        format!("<div class=\"truncate text-xs ink3\">{}</div>", esc(subtitle))
    };
    format!(
        "<header class=\"flex items-center gap-2 border-b line px-3 py-2.5 md:px-4\">{back}\
<div class=\"min-w-0 flex-1\"><div class=\"truncate text-[15px] font-bold leading-tight\">{title}</div>{sub}</div>\
<div class=\"flex items-center gap-1.5\">{actions}</div></header>",
        back = back_btn,
        title = esc(title),
        sub = sub,
        actions = actions
    )
}

pub fn scroll_area(id: &str, inner: &str, extra: &str) -> String {
    format!(
        "<div id=\"{id}\" class=\"min-h-0 flex-1 overflow-y-auto overscroll-contain {extra}\">{inner}</div>"
    )
}

pub fn toast(kind: &str, msg: &str) -> String {
    let ic = match kind {
        "error" => "warning",
        "ok" => "check",
        _ => "info",
    };
    let color = match kind {
        "error" => "#ef4444",
        "ok" => "#10b981",
        _ => "#5333f4",
    };
    format!(
        "<div class=\"toast pointer-events-auto\" role=\"status\"><span style=\"color:{color}\" class=\"mt-0.5\">{}</span><div class=\"flex-1 text-[13px] leading-snug\">{}</div></div>",
        icon(ic, "h-4 w-4"),
        esc(msg)
    )
}

pub fn modal(id: &str, title: &str, body: &str, footer: &str) -> String {
    format!(
        "<div class=\"modal-backdrop\" id=\"{id}\" data-modal>\
<div class=\"modal-card max-h-[88dvh] flex flex-col\">\
<div class=\"flex items-start gap-3 border-b line px-5 py-4\">\
<div class=\"flex-1 text-[15px] font-bold\">{}</div>\
<button class=\"btn btn-ghost btn-icon\" data-close type=\"button\" aria-label=\"Close\">{}</button></div>\
<div class=\"min-h-0 flex-1 overflow-y-auto px-5 py-4\">{}</div>\
<div class=\"flex items-center justify-end gap-2 border-t line px-5 py-3.5\">{}</div>\
</div></div>",
        esc(title),
        icon("x", "h-4 w-4"),
        body,
        footer
    )
}

pub fn field(label: &str, input: &str, hint: &str) -> String {
    let h = if hint.is_empty() {
        String::new()
    } else {
        format!("<div class=\"hint mt-1.5\">{}</div>", esc(hint))
    };
    format!(
        "<label class=\"block\"><span class=\"mb-1.5 block text-xs font-semibold ink2\">{}</span>{}{}</label>",
        esc(label), input, h
    )
}

pub fn spinner(cls: &str) -> String {
    format!(
        "<span class=\"inline-block h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent {}\" style=\"color:var(--color-brand-600)\" aria-hidden=\"true\"></span>",
        cls
    )
}

pub fn rel_ago(ms: i64) -> String {
    rel_time(ms)
}

/// Escape untrusted message text, linkify URLs and turn newlines into <br>.
pub fn render_body(src: &str) -> String {
    let escaped = esc(src);
    let mut out = String::with_capacity(escaped.len());
    let bytes = escaped.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'h' {
            let rest = &escaped[i..];
            let candidate = if rest.starts_with("https://") {
                Some(8)
            } else if rest.starts_with("http://") {
                Some(7)
            } else {
                None
            };
            if let Some(scheme_len) = candidate {
                let mut end = i + scheme_len;
                let mut in_paren = 0i32;
                while end < escaped.len() {
                    let c = escaped.as_bytes()[end];
                    match c {
                        b'(' => in_paren += 1,
                        b')' => {
                            if in_paren == 0 {
                                break;
                            }
                            in_paren -= 1;
                        }
                        b' ' | b'\t' | b'\n' | b'<' | b'"' | b'\'' | b'`' => break,
                        _ => {}
                    }
                    end += 1;
                }
                let url = &escaped[i..end];
                out.push_str(&format!(
                    "<a href=\"{}\" target=\"_blank\" rel=\"noreferrer noopener\">{}</a>",
                    url, url
                ));
                i = end;
                continue;
            }
        }
        if bytes[i] == b'\n' {
            out.push_str("<br>");
            i += 1;
            continue;
        }
        let ch = escaped[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}
