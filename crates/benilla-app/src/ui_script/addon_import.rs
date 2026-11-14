//! The browser's add-on importer, reached from the game: an "Import" button on the character
//! screen's AddOns list (glue) and an "AddOns" button in the Escape menu (in game). Both open the
//! page's importer (`/addons.html?embedded=1` in a dialog, `web/index.html`), which keeps the
//! player's addons in this browser only. An imported addon loads at the next page load.

use benilla_ui::script::UiScript;

/// The `LaunchURL` the glue button asks for; [`super::server_alert::open_launched`] maps it to
/// the importer on the page. Letters, digits, `.`, `-` and `/` only, as the verb's gate wants.
pub(crate) const IMPORT_URL: &str = "http://www.worldofwarcraft.com/wowserv/addons";

/// The in-game global the Escape menu's button calls: the in-game state has no `LaunchURL`.
const OPEN_GLOBAL: &str = "WowservOpenAddOns";

/// Add "Import" beside the AddOns list's Enable/Disable All buttons.
#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn install_glue_import_button(script: &UiScript) -> mlua::Result<()> {
    script.run(&format!(
        r#"
        if AddonList and not AddonListImportButton then
            local ok, b = pcall(CreateFrame, "Button", "AddonListImportButton", AddonList, "GlueButtonSmallTemplate")
            if not ok or not b then
                b = CreateFrame("Button", "AddonListImportButton", AddonList)
            end
            local fr = GetLocale and GetLocale() == "frFR"
            b:SetText(fr and "Importer" or "Import")
            b:SetWidth(120)
            b:SetHeight(22)
            local anchor = AddonListDisableAllButton or AddonListEnableAllButton
            if anchor then
                b:SetPoint("LEFT", anchor, "RIGHT", 8, 0)
            else
                b:SetPoint("BOTTOMLEFT", AddonList, "BOTTOMLEFT", 16, 16)
            end
            b:SetScript("OnClick", function()
                if PlaySound then pcall(PlaySound, "gsTitleOptionOK") end
                LaunchURL("{IMPORT_URL}")
            end)
        end
        "#
    ))
}

/// Add "AddOns" to the Escape menu, between Macros and Logout, and seat [`OPEN_GLOBAL`].
#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn install_game_menu_button(script: &UiScript) -> mlua::Result<()> {
    #[cfg(target_arch = "wasm32")]
    {
        let lua = script.lua();
        lua.globals().set(
            OPEN_GLOBAL,
            lua.create_function(|_, _: mlua::MultiValue| {
                open_importer();
                Ok(())
            })?,
        )?;
    }
    script.run(&format!(
        r#"
        if GameMenuFrame and GameMenuButtonLogout and not GameMenuButtonAddOns then
            local ok, b = pcall(CreateFrame, "Button", "GameMenuButtonAddOns", GameMenuFrame, "GameMenuButtonTemplate")
            if not ok or not b then
                b = CreateFrame("Button", "GameMenuButtonAddOns", GameMenuFrame)
                b:SetWidth(144)
                b:SetHeight(21)
            end
            b:SetText(ADDONS or "AddOns")
            local point, relative, relativePoint, x, y = GameMenuButtonLogout:GetPoint()
            relative = relative or GameMenuButtonMacros
            b:ClearAllPoints()
            b:SetPoint("TOP", relative, "BOTTOM", 0, -1)
            GameMenuButtonLogout:ClearAllPoints()
            GameMenuButtonLogout:SetPoint(point or "TOP", b, relativePoint or "BOTTOM", x or 0, y or -16)
            local h = b:GetHeight()
            if not h or h <= 0 then h = 21 end
            GameMenuFrame:SetHeight(GameMenuFrame:GetHeight() + h + 1)
            b:SetScript("OnClick", function()
                if PlaySound then pcall(PlaySound, "igMainMenuOption") end
                if HideUIPanel then pcall(HideUIPanel, GameMenuFrame) else GameMenuFrame:Hide() end
                if {OPEN_GLOBAL} then {OPEN_GLOBAL}() end
            end)
        end
        "#
    ))
}

/// Open the page's importer: its dialog when the page has one, else `/addons.html` in a new tab.
#[cfg(target_arch = "wasm32")]
pub(crate) fn open_importer() {
    use wasm_bindgen::JsCast;
    let Some(window) = web_sys::window() else {
        return;
    };
    let dialog = js_sys::Reflect::get(&window, &"__wenilla_open_addons".into())
        .ok()
        .and_then(|f| f.dyn_into::<js_sys::Function>().ok());
    match dialog {
        Some(f) => {
            let _ = f.call0(&window);
        }
        None => {
            let _ = window.open_with_url_and_target("/addons.html", "_blank");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_addon_list_gets_an_import_button_that_launches_the_importer() {
        let mut script = UiScript::new_glue().unwrap();
        script
            .run(
                r#"AddonList = CreateFrame("Frame", "AddonList")
                   CreateFrame("Button", "AddonListDisableAllButton", AddonList)"#,
            )
            .unwrap();
        install_glue_import_button(&script).unwrap();
        assert!(script
            .eval::<bool>(
                r#"local _, relative = AddonListImportButton:GetPoint()
                   return relative == AddonListDisableAllButton"#,
            )
            .unwrap());
        script.run("AddonListImportButton:Click()").unwrap();
        assert!(script.take_glue_requests().iter().any(|request| matches!(
            request,
            benilla_ui::script::GlueRequest::LaunchUrl(url) if url == IMPORT_URL
        )));
    }

    #[test]
    fn the_game_menu_gets_an_addons_button_between_macros_and_logout() {
        let script = UiScript::new().unwrap();
        script
            .run(
                r#"GameMenuFrame = CreateFrame("Frame", "GameMenuFrame")
                   GameMenuFrame:SetHeight(250)
                   CreateFrame("Button", "GameMenuButtonMacros", GameMenuFrame)
                   CreateFrame("Button", "GameMenuButtonLogout", GameMenuFrame)
                   GameMenuButtonLogout:SetPoint("TOP", GameMenuButtonMacros, "BOTTOM", 0, -16)
                   opened = 0
                   WowservOpenAddOns = function() opened = opened + 1 end"#,
            )
            .unwrap();
        install_game_menu_button(&script).unwrap();
        assert!(script
            .eval::<bool>(
                r#"local _, a = GameMenuButtonAddOns:GetPoint()
                   local p, b, rp, x, y = GameMenuButtonLogout:GetPoint()
                   return a == GameMenuButtonMacros and b == GameMenuButtonAddOns
                      and p == "TOP" and rp == "BOTTOM" and y == -16
                      and GameMenuFrame:GetHeight() > 250"#,
            )
            .unwrap());
        script.run("GameMenuButtonAddOns:Click()").unwrap();
        assert_eq!(script.eval::<i64>("return opened").unwrap(), 1);
    }
}
