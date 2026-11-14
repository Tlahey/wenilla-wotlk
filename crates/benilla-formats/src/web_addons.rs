//! The page's `Interface\AddOns` folder. The browser build has no filesystem, so the page builds
//! one before the client starts (`web/addons-store.js`, `window.__wenilla_addons`): the addons the
//! player imported into this browser (IndexedDB, never uploaded) over the ones the host serves
//! read-only under `/addons/` (`wenilla-host --addons`). This module is the client's synchronous
//! view of it; every path is relative to the AddOns root, `/` or `\`, any case.
//!
//! Off the page there is no such store: the native client reads its AddOns folder from disk.

/// The addon folders the page knows, each with its own `<Name>.toc`, as their folders spell them.
#[cfg(not(target_arch = "wasm32"))]
pub fn folders() -> Vec<String> {
    Vec::new()
}

/// One file under the AddOns root, or `None` when the page has no such file.
#[cfg(not(target_arch = "wasm32"))]
pub fn read(_rel: &str) -> Option<Vec<u8>> {
    None
}

/// Whether the page has the file under the AddOns root, without reading it.
#[cfg(not(target_arch = "wasm32"))]
pub fn exists(_rel: &str) -> bool {
    false
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use wasm_bindgen::{JsCast, JsValue};

    fn call(method: &str, arg: Option<&str>) -> Option<JsValue> {
        let window = web_sys::window()?;
        let store = js_sys::Reflect::get(&window, &"__wenilla_addons".into()).ok()?;
        if !store.is_object() {
            return None;
        }
        let f: js_sys::Function = js_sys::Reflect::get(&store, &method.into())
            .ok()?
            .dyn_into()
            .ok()?;
        match arg {
            Some(a) => f.call1(&store, &a.into()).ok(),
            None => f.call0(&store).ok(),
        }
    }

    pub fn folders() -> Vec<String> {
        call("folders", None)
            .filter(js_sys::Array::is_array)
            .map(|v| {
                js_sys::Array::from(&v)
                    .iter()
                    .filter_map(|n| n.as_string())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn read(rel: &str) -> Option<Vec<u8>> {
        let bytes = call("read", Some(rel))?
            .dyn_into::<js_sys::Uint8Array>()
            .ok()?;
        Some(bytes.to_vec())
    }

    pub fn exists(rel: &str) -> bool {
        call("has", Some(rel)).is_some_and(|v| v.is_truthy())
    }
}

#[cfg(target_arch = "wasm32")]
pub use wasm::{exists, folders, read};
