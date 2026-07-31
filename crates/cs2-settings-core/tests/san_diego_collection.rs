use cs2_settings_core::{
    TextureSetOverride, scan_export_folder, scan_export_folder_with_overrides,
};
use std::path::PathBuf;

#[test]
fn sign_texture_override_resolves_shared_sign_assets_without_affecting_local_sets() {
    let root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Test Files/San Diego Naval Hospital");
    if !root.is_dir() {
        eprintln!("San Diego reference collection is unavailable; skipping local regression test.");
        return;
    }

    let initial = scan_export_folder(&root).expect("San Diego collection should scan");
    let sign_texture_set = initial
        .texture_sets
        .iter()
        .find(|texture_set| {
            texture_set.name == "SDNH Sign"
                && texture_set.tier == cs2_settings_core::TextureTier::Main
        })
        .expect("SDNH Sign texture set should exist");
    let overrides = initial
        .assets
        .iter()
        .filter(|asset| main_material(asset) == Some("SDNH Signs"))
        .filter(|asset| {
            !matches!(
                asset.main_texture_set.as_ref(),
                Some(texture_set)
                    if texture_set.folder == asset.folder && texture_set.name == asset.name
            )
        })
        .map(|asset| TextureSetOverride {
            asset_folder: asset.folder.clone(),
            texture_set_folder: sign_texture_set.folder.clone(),
            texture_set_name: sign_texture_set.name.clone(),
        })
        .collect::<Vec<_>>();

    assert_eq!(overrides.len(), 6, "six non-provider signs need overrides");

    let resolved = scan_export_folder_with_overrides(&root, &overrides)
        .expect("San Diego overrides should scan");
    let sign_assets = resolved
        .assets
        .iter()
        .filter(|asset| main_material(asset) == Some("SDNH Signs"))
        .collect::<Vec<_>>();
    assert_eq!(sign_assets.len(), 7);
    for asset in sign_assets {
        assert_eq!(
            asset.main_texture_set.as_ref().map(|set| set.name.as_str()),
            Some("SDNH Sign"),
            "asset: {}",
            asset.name
        );
        assert!(asset.settings.can_generate, "asset: {}", asset.name);
        assert!(
            asset
                .issues
                .iter()
                .all(|issue| issue.code != "mainTextureSetUnresolved"),
            "asset: {}",
            asset.name
        );
    }

    let ambulance_sign = resolved
        .assets
        .iter()
        .find(|asset| asset.name == "SDNH Ambulance Sign")
        .expect("ambulance sign should exist");
    assert_eq!(ambulance_sign.settings.entries.len(), 10);
    assert!(
        ambulance_sign
            .settings
            .entries
            .iter()
            .all(|entry| { entry.shared_from.starts_with("../SDNH Sign/SDNH Sign_") })
    );

    let crematorium = resolved
        .assets
        .iter()
        .find(|asset| asset.name == "San Diego Crematorium")
        .expect("crematorium should exist");
    assert_eq!(
        crematorium
            .main_texture_set
            .as_ref()
            .map(|set| set.name.as_str()),
        Some("San Diego Crematorium")
    );
}

fn main_material(asset: &cs2_settings_core::AssetScan) -> Option<&str> {
    asset
        .files
        .iter()
        .find(|file| file.kind == cs2_settings_core::FbxKind::Main)
        .and_then(|file| file.material_names.first().map(String::as_str))
}
