use benilla_ui::{
    framexml, loader,
    script::{AddOnInfo, UiScript},
};

#[test]
fn addon_files_receive_their_name_and_share_a_private_namespace() {
    let script = UiScript::new().unwrap();
    script
        .run_chunk_named(
            b"local name, private = ...; assert(name == 'Probe'); private.value = 42",
            "@Interface\\AddOns\\Probe\\First.lua",
        )
        .unwrap();
    script
        .run_chunk_named(
            b"local name, private = ...; assert(name == 'Probe'); assert(private.value == 42)",
            "@Interface\\AddOns\\Probe\\Second.lua",
        )
        .unwrap();
    script
        .run_chunk_named(
            b"local name, private = ...; assert(name == 'Other'); assert(private.value == nil)",
            "@Interface\\AddOns\\Other\\First.lua",
        )
        .unwrap();
}

#[test]
fn xml_scripts_and_nested_includes_receive_the_same_addon_namespace() {
    let script = UiScript::new().unwrap();
    let doc = framexml::parse(
        r#"<Ui><Script file="First.lua"/><Include file="Nested/Includes.xml"/>
        <Script>local name, private = ...; assert(name == 'Probe'); assert(private.value == 42)</Script>
        </Ui>"#,
    )
    .unwrap();
    let provider = |path: &str| {
        Some(match path {
            "Interface/AddOns/Probe/First.lua" => {
                b"local name, private = ...; assert(name == 'Probe'); private.value = 41".to_vec()
            }
            "Interface/AddOns/Probe/Nested/Includes.xml" => {
                br#"<Ui><Script file="Second.lua"/></Ui>"#.to_vec()
            }
            "Interface/AddOns/Probe/Nested/Second.lua" => {
                b"local name, private = ...; assert(name == 'Probe'); private.value = private.value + 1".to_vec()
            }
            _ => return None,
        })
    };
    let report = loader::load_in(
        &script,
        &doc,
        "Interface/AddOns/Probe/Embeds.xml",
        &provider,
    );
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    assert!(
        report.missing_files.is_empty(),
        "{:?}",
        report.missing_files
    );
}

#[test]
fn core_files_receive_no_addon_arguments() {
    let script = UiScript::new().unwrap();
    script
        .run_chunk_named(
            b"assert(select('#', ...) == 0)",
            "@Interface\\FrameXML\\Core.lua",
        )
        .unwrap();
}

#[test]
fn demand_loaded_files_receive_their_addon_arguments() {
    let mut script = UiScript::new().unwrap();
    script.set_addon_chain_reader(Box::new(|path| {
        Some(match path {
            "Interface/AddOns/Demand/First.lua" => {
                b"local name, private = ...; assert(name == 'Demand'); private.value = 42".to_vec()
            }
            "Interface/AddOns/Demand/Second.lua" => {
                b"local name, private = ...; assert(name == 'Demand'); assert(private.value == 42); DemandReady = true".to_vec()
            }
            _ => return None,
        })
    }));
    script.register_addons(
        vec![AddOnInfo {
            name: "Demand".into(),
            interface: 30300,
            load_on_demand: true,
            chain: true,
            files: vec!["First.lua".into(), "Second.lua".into()],
            enabled: true,
            ..Default::default()
        }],
        None,
        None,
        None,
    );
    script.note_addon_info_reply(&[]);
    script
        .run("assert(LoadAddOn('Demand') == 1); assert(DemandReady)")
        .unwrap();
    assert!(script.errors().is_empty(), "{:?}", script.errors());
}
