use cs2_settings_core::{
    TextureOptions, TextureSelection, TextureSetOverride,
    generate_settings_files_with_texture_options, scan_export_folder,
    scan_export_folder_with_texture_options,
};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

struct Project(PathBuf);
impl Project {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "cs2-project-textures-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        Self(fs::canonicalize(root).unwrap())
    }
    fn texture(&self, folder: &str, name: &str) {
        let folder = self.0.join(folder);
        fs::create_dir_all(&folder).unwrap();
        // The scanner reads the PNG signature and IHDR dimensions only.
        let mut header = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        header.extend(512u32.to_be_bytes());
        header.extend(512u32.to_be_bytes());
        fs::write(folder.join(format!("{name}_BaseColor.png")), header).unwrap();
    }
    fn asset(&self, group: &str, level: &str) -> PathBuf {
        let name = format!("{group} {level}");
        let folder = self.0.join(group).join(&name);
        fs::create_dir_all(&folder).unwrap();
        for suffix in ["", "_LOD1", "_LOD2"] {
            let mesh = format!("{name}{suffix}");
            fs::write(
                folder.join(format!("{mesh}.fbx")),
                format!(
                    r#"; FBX 7.4.0 project file
FBXHeaderExtension: {{ FBXVersion: 7400 }}
Objects: {{
    Geometry: 1, "Geometry::{mesh}", "Mesh" {{
        Vertices: *9 {{ a: 0,0,0,1,0,0,0,1,0 }}
        PolygonVertexIndex: *3 {{ a: 0,1,-3 }}
    }}
    Model: 2, "Model::{mesh}", "Mesh" {{ }}
    Material: 3, "Material::Shared_Mtl", "" {{ }}
}}
Connections: {{
    C: "OO",1,2
    C: "OO",2,0
    C: "OO",3,2
}}
"#
                ),
            )
            .unwrap();
        }
        folder
    }
    fn options(&self, name: &str) -> TextureOptions {
        TextureOptions {
            project_main: Some(TextureSelection {
                texture_set_folder: self.0.clone(),
                texture_set_name: name.into(),
            }),
            ..Default::default()
        }
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn nested_levels_share_root_main_and_their_own_parent_lod2() {
    let p = Project::new();
    p.texture("", "Shared");
    for group in ["Asset 1", "Asset 2"] {
        p.texture(group, &format!("{group}_LOD2"));
        for level in ["L1", "L3", "L5"] {
            p.asset(group, level);
        }
    }
    let scan = scan_export_folder(&p.0).unwrap();
    assert_eq!(scan.assets.len(), 6);
    for asset in &scan.assets {
        assert!(asset.files.iter().all(|file| file.parse_error.is_none()));
        assert!(asset.settings.can_generate, "{:?}", asset.issues);
        assert_eq!(asset.main_texture_set.as_ref().unwrap().folder, p.0);
        let group = asset.folder.parent().unwrap();
        assert_eq!(asset.lod2_texture_set.as_ref().unwrap().folder, group);
        for entry in &asset.settings.entries {
            assert!(asset.folder.join(&entry.shared_from).is_file());
            assert!(!entry.shared_from.contains('\\'));
        }
        let entries = &asset.settings.entries;
        assert!(entries.iter().any(|e| e.shared_to
            == format!("{}_LOD1_BaseColor.png", asset.name)
            && e.shared_from == "../../Shared_BaseColor.png"));
        assert!(entries.iter().any(|e| e.shared_to
            == format!("{}_LOD2_BaseColor.png", asset.name)
            && e.shared_from.starts_with("../Asset ")));
    }
    let report =
        generate_settings_files_with_texture_options(&p.0, false, &[], &[], &Default::default())
            .unwrap();
    assert_eq!(report.items.len(), 6);
    for asset in scan.assets {
        assert_eq!(
            fs::read_to_string(asset.folder.join("settings.json")).unwrap(),
            asset.settings.json
        );
    }
}

#[test]
fn project_choice_manual_override_and_missing_selection_are_respected() {
    let p = Project::new();
    p.texture("", "Shared");
    p.texture("", "Alternative");
    let a = p.asset("Asset 1", "L1");
    p.asset("Asset 1", "L3");
    let options = p.options("Alternative");
    let overrides = [TextureSetOverride {
        asset_folder: a.clone(),
        texture_set_folder: p.0.clone(),
        texture_set_name: "Shared".into(),
    }];
    let scan = scan_export_folder_with_texture_options(&p.0, &overrides, &[], &options).unwrap();
    for asset in scan.assets {
        assert_eq!(
            asset.main_texture_set.unwrap().name,
            if asset.folder == a {
                "Shared"
            } else {
                "Alternative"
            }
        );
    }
    fs::remove_file(p.0.join("Alternative_BaseColor.png")).unwrap();
    let scan = scan_export_folder_with_texture_options(&p.0, &[], &[], &options).unwrap();
    assert!(
        scan.assets
            .iter()
            .all(|a| !a.settings.can_generate && a.main_texture_set.is_none())
    );
    assert!(
        scan_export_folder(&p.0).unwrap().assets.iter().all(|a| a
            .main_texture_set
            .as_ref()
            .unwrap()
            .name
            == "Shared")
    );
    // A selected source outside the scan boundary is never accepted.
    let outside = TextureOptions {
        project_main: Some(TextureSelection {
            texture_set_folder: Path::new("/outside").into(),
            texture_set_name: "Shared".into(),
        }),
        ..Default::default()
    };
    assert!(
        scan_export_folder_with_texture_options(&p.0, &[], &[], &outside)
            .unwrap()
            .assets
            .iter()
            .all(|a| !a.settings.can_generate)
    );
}

#[test]
fn lod2_stays_in_its_group_and_ambiguity_requires_manual_selection() {
    let p = Project::new();
    p.texture("", "Shared");
    p.texture("Asset 1", "Asset 1_LOD2");
    let a = p.asset("Asset 1", "L1");
    let b = p.asset("Asset 2", "L1");
    let scan = scan_export_folder(&p.0).unwrap();
    assert!(
        scan.assets
            .iter()
            .find(|asset| asset.folder == b)
            .unwrap()
            .lod2_texture_set
            .is_none()
    );
    p.texture("Asset 1", "Other_LOD2");
    let scan = scan_export_folder(&p.0).unwrap();
    assert!(
        !scan
            .assets
            .iter()
            .find(|asset| asset.folder == a)
            .unwrap()
            .settings
            .can_generate
    );
    let options = TextureOptions {
        lod2_overrides: vec![TextureSetOverride {
            asset_folder: a.clone(),
            texture_set_folder: p.0.join("Asset 1"),
            texture_set_name: "Other".into(),
        }],
        ..Default::default()
    };
    let scan = scan_export_folder_with_texture_options(&p.0, &[], &[], &options).unwrap();
    assert_eq!(
        scan.assets
            .iter()
            .find(|asset| asset.folder == a)
            .unwrap()
            .lod2_texture_set
            .as_ref()
            .unwrap()
            .name,
        "Other"
    );
    // A local exact LOD2 set wins even if its parent has multiple choices.
    let relative = a.strip_prefix(&p.0).unwrap().to_str().unwrap();
    p.texture(relative, "Asset 1 L1_LOD2");
    let scan = scan_export_folder(&p.0).unwrap();
    assert_eq!(
        scan.assets
            .iter()
            .find(|asset| asset.folder == a)
            .unwrap()
            .lod2_texture_set
            .as_ref()
            .unwrap()
            .folder,
        a
    );
}
