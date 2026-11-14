use benilla_ui::script::{AddOnInfo, UiScript};

#[test]
fn questie_compat_initializes_position_and_thread_functions() {
    let Some(root) = std::env::var_os("BENILLA_ADDON_CORPUS")
        .map(std::path::PathBuf::from)
        .map(|root| root.join("Questie-335"))
        .filter(|root| root.is_dir())
    else {
        eprintln!("skipped: BENILLA_ADDON_CORPUS does not contain Questie-335");
        return;
    };
    let mut script = UiScript::new().unwrap();
    script.register_addons(
        vec![AddOnInfo {
            name: "Questie-335".into(),
            interface: 30300,
            directives: vec![("X-WOW_PROJECT_ID".into(), "11".into())],
            enabled: true,
            ..Default::default()
        }],
        None,
        None,
        None,
    );
    script.note_addon_info_reply(&[]);
    script
        .run(
            "UIParent = CreateFrame('Frame', 'UIParent')
             GameFontNormal = CreateFont('GameFontNormal')
             StaticPopupDialogs = {}
             ERR_QUEST_COMPLETE_S = '%s completed'
             DAILY_QUESTS_REMAINING = '%d remaining'",
        )
        .unwrap();
    for file in [
        "Modules/Libs/QuestieLoader.lua",
        "Modules/QuestieCompat.lua",
        "Compat/Compat.lua",
        "Modules/Libs/ThreadLib.lua",
    ] {
        let source = std::fs::read(root.join(file)).unwrap();
        let result = script.run_chunk_named(
            &source,
            &benilla_ui::script::addon_chunk_name("Questie-335", file),
        );
        assert!(result.is_ok(), "{file}: {result:?}");
    }
    script
        .run("assert(type(QuestieCompat.GetCurrentPlayerPosition) == 'function'); assert(type(QuestieLoader:ImportModule('ThreadLib').ThreadError) == 'function')")
        .unwrap();
}
