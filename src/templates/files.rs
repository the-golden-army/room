use crate::models::{Boot, FileDto, FileNode, User, UserDto};
use crate::util::{esc, ext_of, human_bytes};

use super::{avatar, empty_state, icon, skeleton_grid, skeleton_rows, topbar};

pub fn file_dto_pub(f: &FileNode, by_id: &std::collections::HashMap<i64, User>, children: i64) -> FileDto {
    crate::api::file_dto(f, by_id, false, children)
}

pub fn file_icon(name: &str, mime: &str, kind: &str) -> String {
    let (ic, color) = if kind == "folder" {
        ("folder", "#f59e0b")
    } else {
        match mime {
            m if m.starts_with("image/") => ("image", "#10b981"),
            m if m.starts_with("video/") => ("video", "#8b5cf6"),
            m if m.starts_with("audio/") => ("music", "#ec4899"),
            "application/pdf" => ("file-text", "#ef4444"),
            m if m.contains("zip") || m.contains("tar") || m.contains("compressed") => {
                ("archive", "#f6960c")
            }
            m if m.starts_with("text/") => ("file-text", "#0ea5e9"),
            _ => {
                let e = ext_of(name);
                match e.as_str() {
                    "rs" | "js" | "ts" | "py" | "go" | "c" | "cpp" | "h" | "java" | "sh" => {
                        ("code", "#5333f4")
                    }
                    "md" | "txt" => ("file-text", "#0ea5e9"),
                    "doc" | "docx" | "odt" => ("file-text", "#2563eb"),
                    "xls" | "xlsx" | "csv" => ("grid", "#16a34a"),
                    _ => ("file", "#64748b"),
                }
            }
        }
    };
    format!(
        "<span class=\"grid h-9 w-9 shrink-0 place-items-center rounded-xl\" style=\"color:{color};background:color-mix(in srgb, {color} 12%, transparent)\">{}</span>",
        super::icon(ic, "h-[18px] w-[18px]"),
        color = color
    )
}

pub fn list_column(state: &crate::state::AppState, boot: &Boot, used: i64, quota: i64) -> String {
    let header = super::listcol_header(
        "Drive",
        &format!(
            "<button class=\"btn btn-brand btn-xs\" data-act=\"upload\" type=\"button\">{} Upload</button>",
            icon("upload", "h-3.5 w-3.5")
        ),
    );
    let quota = if quota <= 0 { 100.0 } else { quota as f64 };
    let pct = ((used as f64 / quota) * 100.0).clamp(0.0, 100.0);
    let links = format!(
        "<nav class=\"flex flex-col gap-0.5 p-2\">\
<a class=\"navlink\" href=\"/files\" data-nav-files=\"all\">{}<span>All files</span></a>\
<a class=\"navlink\" href=\"/files?trash=1\" data-nav-files=\"trash\">{}<span>Trash</span></a>\
</nav>",
        icon("drive", "h-4 w-4"),
        icon("trash", "h-4 w-4")
    );
    let meter = format!(
        "<div class=\"border-t line px-4 py-3\">\
<div class=\"mb-1.5 flex items-baseline justify-between\"><span class=\"text-[11px] font-semibold ink2\">Storage</span><span class=\"text-[10px] ink3\">{} used</span></div>\
<div class=\"progress\"><i style=\"width:{:.1}%\"></i></div>\
<div class=\"mt-1.5 text-[10px] ink3\">{:.0}% of drive in use</div>\
<div class=\"mt-3 border-t line pt-2.5 text-[10px] leading-relaxed ink3\">Files stay on this machine. Uploads are streamed to disk — large files never load into memory.</div>\
</div>",
        human_bytes(used),
        pct,
        pct
    );
    let _ = state;
    let _ = boot;
    format!("{header}{links}{meter}")
}

pub fn breadcrumb(path: &[FileDto]) -> String {
    let mut out = String::from("<nav class=\"flex items-center gap-1 overflow-x-auto px-4 pt-3 text-[12.5px]\">");
    out.push_str(&format!(
        "<a class=\"shrink-0 font-semibold ink2 hover:underline\" href=\"/files\">{} Drive</a>",
        icon("drive", "h-3.5 w-3.5 inline")
    ));
    for (i, f) in path.iter().enumerate() {
        let last = i + 1 == path.len();
        out.push_str(&format!(
            "<span class=\"shrink-0 ink3\">/</span><a class=\"shrink-0 truncate {} hover:underline\" href=\"/files?parent={}\">{}</a>",
            if last { "font-bold ink" } else { "ink2" },
            f.id,
            esc(&f.name)
        ));
    }
    out.push_str("</nav>");
    out
}

fn row_html(f: &FileDto, selected: bool) -> String {
    let sel = if selected { " active" } else { "" };
    let kind_label = if f.kind == "folder" {
        format!("{} items", f.children)
    } else {
        human_bytes(f.size)
    };
    format!(
        "<div class=\"list-item list-row{sel}\" data-file=\"{id}\" data-kind=\"{kind}\" draggable=\"true\">\
{icon}\
<span class=\"min-w-0 flex-1\"><span class=\"block truncate text-[13.5px] font-medium\">{name}</span><span class=\"block text-[11px] ink3\">{owner} · {updated}</span></span>\
<span class=\"shrink-0 text-[11px] ink3\">{size}</span>\
<button class=\"btn btn-ghost btn-icon !h-8 !w-8 !rounded-lg row-menu\" data-act=\"file-menu\" type=\"button\" aria-label=\"More\">{more}</button>\
</div>",
        sel = sel,
        id = f.id,
        kind = f.kind,
        icon = file_icon(&f.name, &f.mime, &f.kind),
        name = esc(&f.name),
        owner = esc(&f.owner_name),
        updated = super::rel_ago(f.updated_at),
        size = kind_label,
        more = icon("more", "h-4 w-4")
    )
}

fn grid_card_html(f: &FileDto, selected: bool) -> String {
    let sel = if selected { " ring-2\" style=\"--tw-ring-color:var(--color-brand-500)" } else { "" };
    let thumb = if crate::util::is_image(&f.mime) {
        format!(
            "<div class=\"grid h-28 place-items-center overflow-hidden rounded-xl panel2\">\
<img src=\"/api/files/{}/raw?preview=1\" alt=\"{}\" class=\"h-full w-full object-cover\" loading=\"lazy\"></div>",
            f.id,
            esc(&f.name)
        )
    } else {
        format!(
            "<div class=\"grid h-28 place-items-center rounded-xl panel2\">{}</div>",
            file_icon(&f.name, &f.mime, &f.kind)
        )
    };
    format!(
        "<div class=\"tile tile-hover cursor-pointer p-2.5{sel}\" data-file=\"{id}\" data-kind=\"{kind}\" draggable=\"true\" title=\"{name}\">\
{thumb}\
<div class=\"mt-2 flex items-start gap-1.5\">\
<span class=\"min-w-0 flex-1 truncate text-[12.5px] font-medium\">{name}</span>\
<button class=\"btn btn-ghost btn-icon !h-7 !w-7 !rounded-lg row-menu\" data-act=\"file-menu\" type=\"button\" aria-label=\"More\">{more}</button>\
</div>\
<div class=\"mt-0.5 text-[10.5px] ink3\">{size}</div></div>",
        sel = sel,
        id = f.id,
        kind = f.kind,
        thumb = thumb,
        name = esc(&f.name),
        more = icon("more", "h-3.5 w-3.5"),
        size = if f.kind == "folder" { format!("{} items", f.children) } else { human_bytes(f.size) }
    )
}

pub struct FileView {
    pub path: Vec<FileDto>,
    pub items: Vec<FileDto>,
    pub trashed: bool,
    pub view: String,
    pub query: String,
}

pub fn main_column(boot: &Boot, v: &FileView, selected: Option<&FileDto>) -> String {
    let title = if v.trashed {
        "Trash".to_string()
    } else {
        v.path
            .last()
            .map(|f| f.name.clone())
            .unwrap_or_else(|| "All files".to_string())
    };
    let sub = if v.trashed {
        "Deleted items stay here until you empty the trash".to_string()
    } else {
        format!("{} items · {}", v.items.len(), boot.server.name)
    };
    let actions = format!(
        "<button class=\"btn btn-ghost btn-icon\" data-act=\"upload\" title=\"Upload\">{}</button>\
<button class=\"btn btn-ghost btn-icon\" data-act=\"new-folder\" title=\"New folder\">{}</button>\
<button class=\"btn btn-ghost btn-icon\" data-act=\"view-toggle\" title=\"Toggle view\">{}</button>\
<a class=\"btn btn-ghost btn-icon\" href=\"/api/files/zip?parent={}\" title=\"Download folder as zip\">{}</a>",
        icon("upload", "h-4 w-4"),
        icon("folder-plus", "h-4 w-4"),
        icon(if v.view == "grid" { "list" } else { "grid" }, "h-4 w-4"),
        v.path.last().map(|f| f.id).unwrap_or(0),
        icon("download", "h-4 w-4")
    );

    let body = if v.items.is_empty() {
        empty_state(
            if v.trashed { "trash" } else { "folder" },
            if v.trashed { "Trash is empty" } else { "This folder is empty" },
            if v.trashed {
                "Nothing has been deleted recently.".to_string()
            } else {
                "Drop files here, or use the upload button to add documents, photos and archives."
                    .to_string()
            },
            if v.trashed {
                String::new()
            } else {
                format!(
                    "<div class=\"flex gap-2\"><button class=\"btn btn-brand\" data-act=\"upload\" type=\"button\">{} Upload files</button><button class=\"btn btn-ghost\" data-act=\"new-folder\" type=\"button\">{} New folder</button></div>",
                    icon("upload", "h-4 w-4"),
                    icon("folder-plus", "h-4 w-4")
                )
            },
        )
    } else if v.view == "grid" {
        let mut cards = String::new();
        for f in &v.items {
            cards.push_str(&grid_card_html(f, selected.map(|s| s.id) == Some(f.id)));
        }
        format!("<div id=\"file-grid\" class=\"grid grid-cols-2 gap-3 p-4 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5\">{cards}</div><div class=\"hidden\" id=\"drop-hint\">Drop to upload</div>")
    } else {
        let mut rows = String::new();
        rows.push_str(
            "<div class=\"hidden grid-cols-[minmax(0,1fr)_90px_90px_40px] gap-2 px-3 pb-1.5 text-[10.5px] font-bold uppercase tracking-[0.05em] ink3 md:grid\"><span>Name</span><span>Size</span><span>Modified</span><span></span></div>",
        );
        for f in &v.items {
            rows.push_str(&row_html(f, selected.map(|s| s.id) == Some(f.id)));
        }
        format!("<div class=\"flex flex-col gap-0.5 p-2\" id=\"file-list\">{rows}</div>")
    };

    format!(
        "{}{}{}",
        topbar("history.back()", &title, &sub, &actions),
        breadcrumb(&v.path),
        super::scroll_area("files-scroll", &body, "pb-6")
    )
}

pub fn details_panel(f: &FileDto, boot: &Boot, shares: &[(UserDto, String)]) -> String {
    let mut out = String::new();
    out.push_str(&super::listcol_header(
        "Preview",
        &format!(
            "<button class=\"btn btn-ghost btn-icon\" data-act=\"close-panel\" title=\"Close\">{}</button>",
            icon("x", "h-4 w-4")
        ),
    ));
    let mut body = String::new();
    if f.kind == "folder" {
        body.push_str(&format!(
            "<div class=\"grid h-36 place-items-center rounded-2xl panel2\">{}</div>",
            file_icon(&f.name, &f.mime, "folder")
        ));
    } else if crate::util::is_image(&f.mime) {
        body.push_str(&format!(
            "<div class=\"overflow-hidden rounded-2xl panel2\"><img src=\"/api/files/{}/raw?preview=1\" alt=\"{}\" class=\"max-h-56 w-full object-cover\"></div>",
            f.id,
            esc(&f.name)
        ));
    } else {
        body.push_str(&format!(
            "<div class=\"grid h-36 place-items-center rounded-2xl panel2\">{}</div>",
            file_icon(&f.name, &f.mime, &f.kind)
        ));
    }
    body.push_str(&format!(
        "<div class=\"mt-3 px-1\"><div class=\"text-sm font-bold\">{}</div><div class=\"mt-0.5 text-[11.5px] ink3\">{} · {}</div></div>",
        esc(&f.name),
        human_bytes(f.size),
        esc(&f.mime)
    ));
    body.push_str(&format!(
        "<div class=\"mt-3 grid grid-cols-2 gap-2 px-1 text-[11.5px]\"><div class=\"tile p-2.5\"><div class=\"ink3 text-[10px]\">Owner</div><div class=\"mt-0.5 truncate font-semibold\">{}</div></div><div class=\"tile p-2.5\"><div class=\"ink3 text-[10px]\">Modified</div><div class=\"mt-0.5 font-semibold\">{}</div></div></div>",
        esc(&f.owner_name),
        super::rel_ago(f.updated_at)
    ));
    body.push_str(&format!(
        "<div class=\"mt-3 flex flex-wrap gap-2 px-1\">\
<a class=\"btn btn-brand btn-xs\" href=\"/api/files/{id}/download\" title=\"Download\">{dl} Download</a>\
<button class=\"btn btn-ghost btn-xs\" data-act=\"rename\" type=\"button\">{rn} Rename</button>\
<button class=\"btn btn-ghost btn-xs\" data-act=\"move\" type=\"button\">{mv} Move</button>\
<button class=\"btn btn-ghost btn-xs hover:!text-red-500\" data-act=\"delete\" type=\"button\">{del} Delete</button>\
</div>",
        id = f.id,
        dl = icon("download", "h-3.5 w-3.5"),
        rn = icon("edit", "h-3.5 w-3.5"),
        mv = icon("folder", "h-3.5 w-3.5"),
        del = icon("trash", "h-3.5 w-3.5")
    ));
    if f.kind == "folder" {
        body.push_str(&format!(
            "<div class=\"mt-4 px-1\"><div class=\"mb-1.5 text-[11px] font-bold uppercase tracking-[0.06em] ink3\">Shared with</div><div class=\"flex flex-col gap-1\">{}</div></div>",
            if shares.is_empty() {
                "<div class=\"text-[12px] ink3\">Private — only you can see it.</div>".to_string()
            } else {
                shares
                    .iter()
                    .map(|(u, p)| format!(
                        "<div class=\"navlink\">{}<span class=\"min-w-0 flex-1 truncate\">{}</span><span class=\"chip\">{}</span></div>",
                        avatar(u, "h-7 w-7 text-[10px]", false),
                        esc(&u.name),
                        esc(p)
                    ))
                    .collect::<String>()
            }
        ));
    }
    let _ = boot;
    out.push_str(&super::scroll_area("file-details", &body, "p-3"));
    out
}

pub fn skeleton_files() -> String {
    format!(
        "{}{}",
        skeleton_rows(6),
        skeleton_grid(8)
    )
}
