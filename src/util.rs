use rand::RngCore;
use sha2::{Digest, Sha256};

pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

pub fn random_token() -> String {
    let mut buf = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut buf);
    hex::encode(buf)
}

/// A short code a person can read off a console and type on a phone.
/// Avoids look-alike characters (0/O, 1/I/L) so it survives being retyped.
pub fn pairing_code() -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";
    let mut buf = [0u8; 8];
    rand::thread_rng().fill_bytes(&mut buf);
    let chars: String = buf
        .iter()
        .map(|b| ALPHABET[(*b as usize) % ALPHABET.len()] as char)
        .collect();
    format!("{}-{}", &chars[..4], &chars[4..])
}

/// Compare a typed code against the expected one without leaking timing.
pub fn code_matches(expected: &str, given: &str) -> bool {
    let norm = |s: &str| -> String {
        s.chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .map(|c| c.to_ascii_uppercase())
            .collect()
    };
    let (a, b) = (norm(expected), norm(given));
    if a.is_empty() || a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.bytes().zip(b.bytes()) {
        diff |= x ^ y;
    }
    diff == 0
}

pub fn hash_token(token: &str) -> String {
    let mut h = Sha256::new();
    h.update(b"room-device-token::");
    h.update(token.as_bytes());
    hex::encode(h.finalize())
}

pub fn human_bytes(n: i64) -> String {
    const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
    if n < 0 {
        return "0 B".into();
    }
    let mut v = n as f64;
    let mut i = 0usize;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{} {}", n, UNITS[i])
    } else {
        format!("{:.1} {}", v, UNITS[i])
    }
}

pub fn human_duration(ms: i64) -> String {
    if ms < 1000 {
        return "just now".into();
    }
    let s = ms / 1000;
    if s < 60 {
        return format!("{}s ago", s);
    }
    let m = s / 60;
    if m < 60 {
        return format!("{}m ago", m);
    }
    let h = m / 60;
    if h < 24 {
        return format!("{}h ago", h);
    }
    let d = h / 24;
    if d < 7 {
        return format!("{}d ago", d);
    }
    let w = d / 7;
    if w < 5 {
        return format!("{}w ago", w);
    }
    let mo = d / 30;
    if mo < 12 {
        return format!("{}mo ago", mo);
    }
    format!("{}y ago", d / 365)
}

/// Compact duration for call timers: hh:mm:ss / mm:ss
pub fn clock(ms: i64) -> String {
    let total = (ms / 1000).max(0);
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{:02}:{:02}:{:02}", h, m, s)
    } else {
        format!("{:02}:{:02}", m, s)
    }
}

const AVATAR_COLORS: [&str; 12] = [
    "#5333f4", "#0ea5e9", "#10b981", "#f6960c", "#ef4444", "#ec4899", "#8b5cf6", "#14b8a6",
    "#f97316", "#6366f1", "#84cc16", "#0f766e",
];

pub fn avatar_color(seed: &str) -> String {
    let mut h: u64 = 5381;
    for b in seed.bytes() {
        h = h.wrapping_mul(33).wrapping_add(b as u64);
    }
    AVATAR_COLORS[(h % AVATAR_COLORS.len() as u64) as usize].to_string()
}

pub fn initials(name: &str) -> String {
    let mut out = String::new();
    for part in name.split_whitespace().take(2) {
        if let Some(c) = part.chars().next() {
            out.extend(c.to_uppercase());
        }
    }
    if out.is_empty() {
        "?".into()
    } else {
        out
    }
}

/// HTML-escape untrusted text so it can be embedded in markup or attribute values.
pub fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// Validate a JSON payload and return it as a raw JS object literal for embedding
/// inside a `<script>` block (already JSON-escaped, so no extra JS quoting).
pub fn js_literal(json: &str) -> String {
    if serde_json::from_str::<serde_json::Value>(json).is_err() {
        return "{}".to_string();
    }
    let mut out = String::with_capacity(json.len());
    for c in json.chars() {
        match c {
            '<' => out.push_str("\\u003c"),
            '>' => out.push_str("\\u003e"),
            '&' => out.push_str("\\u0026"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            _ => out.push(c),
        }
    }
    out
}
/// Escape for safe inclusion inside a double-quoted JS string literal in a `<script>` block.
pub fn js_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '<' => out.push_str("\\u003c"),
            '>' => out.push_str("\\u003e"),
            '&' => out.push_str("\\u0026"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max).collect();
    out.push('…');
    out
}

/// Keep only characters that are safe for a filename; never allow path separators.
pub fn sanitize_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | '\0' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if (c as u32) < 0x20 => '_',
            c => c,
        })
        .collect();
    let trimmed = cleaned.trim().trim_end_matches('.');
    if trimmed.is_empty() {
        "unnamed".into()
    } else {
        trimmed.chars().take(180).collect()
    }
}

/// Normalize a client-supplied node name (folder/file rename).
pub fn clean_node_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | '\0' => '_',
            c if (c as u32) < 0x20 => '_',
            c => c,
        })
        .collect();
    let t = cleaned.trim();
    if t.is_empty() {
        "Untitled".into()
    } else {
        t.chars().take(120).collect()
    }
}

pub fn is_image(mime: &str) -> bool {
    mime.starts_with("image/")
}

pub fn ext_of(name: &str) -> String {
    name.rsplit_once('.')
        .map(|(_, e)| e.to_lowercase())
        .unwrap_or_default()
}

/// Clamp helper used by pagination.
pub fn clamp_len(v: i64, min: i64, max: i64, default: i64) -> i64 {
    if v <= 0 {
        default
    } else {
        v.clamp(min, max)
    }
}

pub fn client_ip(req: &actix_web::HttpRequest) -> String {
    if let Some(v) = req.headers().get("x-forwarded-for") {
        if let Ok(s) = v.to_str() {
            if let Some(first) = s.split(',').next() {
                let t = first.trim();
                if !t.is_empty() {
                    return t.to_string();
                }
            }
        }
    }
    req.peer_addr()
        .map(|a| a.ip().to_string())
        .unwrap_or_else(|| "-".into())
}

pub fn platform_from_ua(req: &actix_web::HttpRequest) -> String {
    let ua = req
        .headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_lowercase();
    if ua.contains("iphone") || ua.contains("ipad") || ua.contains("ios") {
        "iOS".into()
    } else if ua.contains("android") {
        "Android".into()
    } else if ua.contains("mac os") {
        "macOS".into()
    } else if ua.contains("windows") {
        "Windows".into()
    } else if ua.contains("linux") {
        "Linux".into()
    } else {
        "Web".into()
    }
}

pub fn device_name_from_ua(req: &actix_web::HttpRequest) -> String {
    let ua = req
        .headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_lowercase();
    let os = if ua.contains("iphone") || ua.contains("ipad") {
        "iPad / iPhone"
    } else if ua.contains("android") {
        "Android device"
    } else if ua.contains("mac os") {
        "Mac"
    } else if ua.contains("windows") {
        "Windows PC"
    } else if ua.contains("linux") {
        "Linux PC"
    } else {
        "Browser"
    };
    let browser = if ua.contains("edg/") {
        "Edge"
    } else if ua.contains("chrome") {
        "Chrome"
    } else if ua.contains("safari") {
        "Safari"
    } else if ua.contains("firefox") {
        "Firefox"
    } else {
        "Browser"
    };
    format!("{} · {}", os, browser)
}

/// Same-day clock label (HH:MM) for message headers.
pub fn hhmm(ms: i64) -> String {
    use chrono::{Local, TimeZone};
    match Local.timestamp_millis_opt(ms) {
        chrono::LocalResult::Single(dt) => dt.format("%H:%M").to_string(),
        _ => String::new(),
    }
}
