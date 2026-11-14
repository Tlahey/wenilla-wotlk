//! The Escape menu's Full Screen button: the browser page's own full screen
//! (`window.__wenilla_fullscreen`, `web/index.html`). The click is the user gesture the Fullscreen
//! API needs, which a winit mode change never has (`video::boot_window_mode`).

use benilla_ui::script::UiScript;

/// Add "Full Screen" to the Escape menu just above Logout (below AddOns when that is installed
/// first), and seat `BrowserFullscreen([toggle])`: the page's state before the toggle, nil where
/// it cannot go full screen. The label names what a click will do and is re-read on each show.
pub(super) fn install_game_menu_button(script: &UiScript) -> mlua::Result<()> {
    #[cfg(target_arch = "wasm32")]
    {
        let lua = script.lua();
        lua.globals().set(
            "BrowserFullscreen",
            lua.create_function(|_, toggle: Option<bool>| Ok(browser_fullscreen(toggle.unwrap_or(false))))?,
        )?;
    }
    script.run(
        r#"
        if GameMenuFrame and GameMenuButtonLogout and not GameMenuButtonFullscreen then
            local ok, b = pcall(CreateFrame, "Button", "GameMenuButtonFullscreen", GameMenuFrame, "GameMenuButtonTemplate")
            if not ok or not b then
                b = CreateFrame("Button", "GameMenuButtonFullscreen", GameMenuFrame)
                b:SetWidth(144)
                b:SetHeight(21)
            end
            local point, relative, relativePoint, x, y = GameMenuButtonLogout:GetPoint()
            relative = relative or GameMenuButtonMacros
            b:ClearAllPoints()
            b:SetPoint("TOP", relative, "BOTTOM", 0, -1)
            GameMenuButtonLogout:ClearAllPoints()
            GameMenuButtonLogout:SetPoint(point or "TOP", b, relativePoint or "BOTTOM", x or 0, y or -16)
            local h = b:GetHeight()
            if not h or h <= 0 then h = 21 end
            GameMenuFrame:SetHeight(GameMenuFrame:GetHeight() + h + 1)
            local function refresh()
                local full = BrowserFullscreen and BrowserFullscreen()
                local fr = GetLocale and GetLocale() == "frFR"
                if full then
                    b:SetText(fr and "Mode fenêtré" or "Windowed Mode")
                else
                    b:SetText(fr and "Plein écran" or "Full Screen")
                end
                if full == nil then b:Disable() else b:Enable() end
            end
            refresh()
            b:SetScript("OnShow", refresh)
            b:SetScript("OnClick", function()
                if PlaySound then pcall(PlaySound, "igMainMenuOption") end
                if HideUIPanel then pcall(HideUIPanel, GameMenuFrame) else GameMenuFrame:Hide() end
                if BrowserFullscreen then BrowserFullscreen(true) end
            end)
        end
        "#,
    )
}

#[cfg(target_arch = "wasm32")]
fn browser_fullscreen(toggle: bool) -> Option<bool> {
    use wasm_bindgen::JsCast;
    let window = web_sys::window()?;
    let hook = js_sys::Reflect::get(&window, &"__wenilla_fullscreen".into()).ok()?;
    let state = hook
        .dyn_into::<js_sys::Function>()
        .ok()?
        .call1(&window, &toggle.into())
        .ok()?
        .as_f64()?;
    (state >= 0.0).then_some(state == 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_game_menu_gets_a_full_screen_button_that_follows_the_page() {
        let mut script = UiScript::new().unwrap();
        script
            .run(
                r#"GameMenuFrame = CreateFrame("Frame", "GameMenuFrame")
                   GameMenuFrame:SetHeight(250)
                   CreateFrame("Button", "GameMenuButtonAddOns", GameMenuFrame)
                   CreateFrame("Button", "GameMenuButtonLogout", GameMenuFrame)
                   GameMenuButtonLogout:SetPoint("TOP", GameMenuButtonAddOns, "BOTTOM", 0, -16)
                   pageFull, toggles = false, 0
                   BrowserFullscreen = function(toggle)
                       if toggle then toggles = toggles + 1 end
                       return pageFull
                   end"#,
            )
            .unwrap();
        install_game_menu_button(&script).unwrap();
        assert!(script
            .eval::<bool>(
                r#"local _, a = GameMenuButtonFullscreen:GetPoint()
                   local _, b, _, _, y = GameMenuButtonLogout:GetPoint()
                   return a == GameMenuButtonAddOns and b == GameMenuButtonFullscreen and y == -16
                      and GameMenuButtonFullscreen:GetText() == "Full Screen"
                      and GameMenuFrame:GetHeight() > 250"#,
            )
            .unwrap());
        script.run("GameMenuButtonFullscreen:Click()").unwrap();
        assert_eq!(script.eval::<i64>("return toggles").unwrap(), 1);
        script.run("pageFull = true GameMenuFrame:Hide()").unwrap();
        script.resolve();
        script.run("GameMenuFrame:Show()").unwrap();
        script.resolve();
        assert_eq!(
            script.eval::<String>("return GameMenuButtonFullscreen:GetText()").unwrap(),
            "Windowed Mode"
        );
    }
}
