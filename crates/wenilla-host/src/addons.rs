//! `GET /addons/__catalog` and `GET /addons/{*path}` — the admin's addons, served read-only from
//! one host folder (`--addons`). The page lists them in its AddOns folder (`web/addons-store.js`)
//! beside the ones the player imported into the browser; nothing is ever written here.
//!
//! Only folders holding their own `<Name>.toc` are addons, as the client's loose pass sees them;
//! hidden entries (`.git`, `.DS_Store`) and anything resolving outside the folder are refused.

use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use axum::extract::State;
use axum::http::{header, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;

/// The most files one catalog lists; past it the walk stops.
pub const MAX_FILES: usize = 20_000;
/// The largest file served.
pub const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Clone)]
struct Addons {
    root: Arc<PathBuf>,
}

/// The `/addons/*` router over `dir`.
pub fn router(dir: PathBuf) -> Router {
    let root = dir.canonicalize().unwrap_or(dir);
    Router::new()
        .route("/addons/__catalog", get(catalog_route))
        .route("/addons/{*path}", get(file_route))
        .with_state(Addons {
            root: Arc::new(root),
        })
}

/// Every file of every addon folder under `root`, `/`-separated and relative to it, sorted.
pub fn catalog(root: &Path) -> Vec<String> {
    let root = &root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let mut files = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return files;
    };
    let mut folders: Vec<(String, PathBuf)> = entries
        .flatten()
        .filter_map(|e| Some((e.file_name().to_str()?.to_owned(), e.path())))
        .filter(|(name, path)| !name.starts_with('.') && path.is_dir())
        .filter(|(name, path)| has_toc(path, name))
        .collect();
    folders.sort();
    for (name, path) in folders {
        walk(root, &path, &name, &mut files);
    }
    files.sort();
    files
}

fn has_toc(dir: &Path, name: &str) -> bool {
    let want = format!("{name}.toc");
    std::fs::read_dir(dir).is_ok_and(|entries| {
        entries.flatten().any(|e| {
            e.file_name()
                .to_str()
                .is_some_and(|f| f.eq_ignore_ascii_case(&want))
                && e.path().is_file()
        })
    })
}

fn walk(root: &Path, dir: &Path, rel: &str, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if out.len() >= MAX_FILES {
            return;
        }
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        if !inside(root, &path) {
            continue;
        }
        let child = format!("{rel}/{name}");
        if path.is_dir() {
            walk(root, &path, &child, out);
        } else if path
            .metadata()
            .is_ok_and(|m| m.is_file() && m.len() <= MAX_FILE_BYTES)
        {
            out.push(child);
        }
    }
}

/// Whether `path`, symlinks resolved, is still under `root` (already canonical).
fn inside(root: &Path, path: &Path) -> bool {
    path.canonicalize().is_ok_and(|p| p.starts_with(root))
}

/// `rel` under `root`, each component matched without regard to ASCII case; `None` for a dot or
/// hidden component, a path outside the root, or a file in a folder that is not an addon.
pub fn resolve(root: &Path, rel: &str) -> Option<PathBuf> {
    let rel = Path::new(rel);
    let mut parts = Vec::new();
    for c in rel.components() {
        match c {
            Component::Normal(s) => {
                let s = s.to_str()?;
                if s.is_empty() || s.starts_with('.') {
                    return None;
                }
                parts.push(s);
            }
            _ => return None,
        }
    }
    if parts.len() < 2 {
        return None;
    }
    let mut at = root.to_path_buf();
    for (i, part) in parts.iter().enumerate() {
        let exact = at.join(part);
        at = if exact.exists() {
            exact
        } else {
            std::fs::read_dir(&at)
                .ok()?
                .flatten()
                .find(|e| e.file_name().to_str().is_some_and(|n| n.eq_ignore_ascii_case(part)))?
                .path()
        };
        if i == 0 {
            let folder = at.file_name()?.to_str()?.to_owned();
            if !has_toc(&at, &folder) {
                return None;
            }
        }
    }
    (inside(root, &at) && at.is_file()).then_some(at)
}

async fn catalog_route(State(addons): State<Addons>) -> Response {
    let root = Arc::clone(&addons.root);
    let files = tokio::task::spawn_blocking(move || catalog(&root))
        .await
        .unwrap_or_default();
    (
        [
            (header::CONTENT_TYPE, "application/json"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        serde_json::json!({ "files": files }).to_string(),
    )
        .into_response()
}

async fn file_route(uri: Uri, State(addons): State<Addons>) -> Response {
    let raw = uri.path().strip_prefix("/addons/").unwrap_or_default();
    let Ok(rel) = percent_encoding::percent_decode_str(raw).decode_utf8() else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let rel = rel.replace('\\', "/");
    let root = Arc::clone(&addons.root);
    let read = tokio::task::spawn_blocking(move || {
        let path = resolve(&root, &rel)?;
        if path.metadata().ok()?.len() > MAX_FILE_BYTES {
            return None;
        }
        std::fs::read(path).ok()
    })
    .await
    .ok()
    .flatten();
    match read {
        Some(bytes) => (
            [
                (header::CONTENT_TYPE, "application/octet-stream"),
                (header::CACHE_CONTROL, "no-cache"),
            ],
            bytes,
        )
            .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(tag: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("wenilla_addons_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for (path, body) in [
            ("Atlas/Atlas.toc", "## Title: Atlas"),
            ("Atlas/Images/Map.blp", "BLP2"),
            ("Atlas/.git/config", "secret"),
            ("NotAnAddon/readme.txt", "x"),
        ] {
            let file = root.join(path);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, body).unwrap();
        }
        root.canonicalize().unwrap()
    }

    #[test]
    fn the_catalog_lists_only_addon_folders_and_skips_hidden_files() {
        let root = tree("catalog");
        assert_eq!(
            catalog(&root),
            vec!["Atlas/Atlas.toc".to_string(), "Atlas/Images/Map.blp".to_string()]
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn files_resolve_case_insensitively_and_never_escape() {
        let root = tree("resolve");
        assert!(resolve(&root, "atlas/images/MAP.blp").is_some());
        assert!(resolve(&root, "Atlas/.git/config").is_none());
        assert!(resolve(&root, "NotAnAddon/readme.txt").is_none());
        assert!(resolve(&root, "Atlas/../NotAnAddon/readme.txt").is_none());
        assert!(resolve(&root, "/etc/passwd").is_none());
        let _ = std::fs::remove_dir_all(root);
    }
}
