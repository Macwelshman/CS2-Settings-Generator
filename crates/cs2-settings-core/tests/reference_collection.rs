use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

#[test]
fn supplied_reference_settings_are_reproduced_when_available() {
    let Some(root) = std::env::var_os("CS2_REFERENCE_ROOT").map(PathBuf::from) else {
        eprintln!("CS2_REFERENCE_ROOT not set; skipping local reference comparison.");
        return;
    };

    let scan = cs2_settings_core::scan_export_folder(&root).expect("reference scan should succeed");
    assert_eq!(scan.assets.len(), 10);

    for asset in scan.assets {
        let existing_path = asset.folder.join("settings.json");
        let existing: serde_json::Value =
            serde_json::from_slice(&fs::read(&existing_path).expect("reference settings missing"))
                .expect("reference settings should be valid JSON");
        let expected = existing
            .get("sharedAssets")
            .and_then(serde_json::Value::as_object)
            .expect("reference settings should contain sharedAssets");
        let generated = asset
            .settings
            .entries
            .iter()
            .map(|entry| (entry.shared_to.clone(), entry.shared_from.clone()))
            .collect::<BTreeMap<_, _>>();
        let expected = expected
            .iter()
            .map(|(key, value)| {
                (
                    key.clone(),
                    value
                        .as_str()
                        .expect("shared asset value should be a string")
                        .to_owned(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        assert_eq!(generated, expected, "asset: {}", asset.name);
    }
}
