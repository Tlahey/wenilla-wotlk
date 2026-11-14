//! Fork-only browser export of the layer's error log; no player data is uploaded.

use benilla_ui::script::UiScript;

#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn install(script: &UiScript) -> mlua::Result<()> {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        script.lua().globals().set(
            "WowservCopyScriptErrors",
            script
                .lua()
                .create_function(|_, (text, locale): (mlua::String, String)| {
                    let text = benilla_ui::source::decode(&text.as_bytes()).into_owned();
                    let window = web_sys::window()
                        .ok_or_else(|| mlua::Error::external("browser window unavailable"))?;
                    let copy = js_sys::Reflect::get(&window, &"__wenilla_copy_errors".into())
                        .map_err(|_| mlua::Error::external("clipboard helper unavailable"))?
                        .dyn_into::<js_sys::Function>()
                        .map_err(|_| mlua::Error::external("clipboard helper unavailable"))?;
                    copy.call2(&window, &text.into(), &locale.into())
                        .map_err(|_| mlua::Error::external("clipboard export failed"))?;
                    Ok(())
                })?,
        )?;
    }
    script.run(
        r#"
        if BenillaScriptLogCopyButton then
            BenillaScriptLogCopyButton:SetText(GetLocale() == "frFR" and "Tout copier" or "Copy all")
            BenillaScriptLogCopyButton:Show()
        end
        "#,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use benilla_ui::framexml::{ScriptRef, TopLevel};

    #[test]
    fn export_keeps_full_messages_counts_order_and_retention_metadata() {
        let script = UiScript::new().unwrap();
        script.run("UIPanelWindows = {}").unwrap();
        let doc = benilla_ui::framexml::parse(include_str!("../../assets/ui/ScriptLogFrame.xml"))
            .unwrap();
        let TopLevel::Script(ScriptRef::Inline { body, .. }) = &doc.items[0] else {
            panic!("log definitions must precede the frames");
        };
        script.run(body).unwrap();
        script
            .run(
                r#"BenillaScriptLog_Record("first\nfull traceback")
                   BenillaScriptLog_Record("second")
                   BenillaScriptLog_Record("first\nfull traceback")
                   BenillaScriptLog.total = 300"#,
            )
            .unwrap();
        let text: String = script.eval("return BenillaScriptLog_Export()").unwrap();
        assert!(text.contains("retained: 2; issued: 300; limit: 256"));
        assert!(text.ends_with("#1 x2\nfirst\nfull traceback\n\n#2 x1\nsecond"));
        script.run("BenillaScriptLog.rows = {}").unwrap();
        let empty: String = script.eval("return BenillaScriptLog_Export()").unwrap();
        assert!(empty.contains("retained: 0; issued: 300"));
        assert!(!empty.contains("full traceback"));
    }

    #[test]
    fn the_copy_button_exports_the_log_without_changing_the_selection() {
        let script = UiScript::new().unwrap();
        script
            .run(
                r#"UIPanelWindows = {}
                   SlashCmdList = {}
                   WowservCopyScriptErrors = function(text, locale)
                       exported = text
                       exportedLocale = locale
                   end"#,
            )
            .unwrap();
        let doc = benilla_ui::framexml::parse(include_str!("../../assets/ui/ScriptLogFrame.xml"))
            .unwrap();
        benilla_ui::loader::load(&script, &doc, &|_| None);
        install(&script).unwrap();
        script
            .run(
                r#"BenillaScriptLog_Record("one")
                   BenillaScriptLog_Record("two")
                   BenillaScriptLogSelected = 1
                   BenillaScriptLogCopyButton:Click()"#,
            )
            .unwrap();
        assert_eq!(
            script.eval::<String>("return exported").unwrap(),
            script
                .eval::<String>("return BenillaScriptLog_Export()")
                .unwrap()
        );
        assert_eq!(
            script
                .eval::<i64>("return BenillaScriptLogSelected")
                .unwrap(),
            1
        );
        assert!(script.errors().is_empty(), "{:?}", script.errors());
    }
}
