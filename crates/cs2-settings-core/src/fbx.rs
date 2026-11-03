use crate::model::{FbxFile, FbxKind, Issue};
use std::path::Path;

const SUFFIXES: &[(&str, FbxKind)] = &[
    ("_LOD1_Win", FbxKind::Lod1Window),
    ("_LOD1_Wio", FbxKind::Lod1WioWindow),
    ("_LOD1_Wim", FbxKind::Lod1WimWindow),
    ("_LOD1_Wif", FbxKind::Lod1WifWindow),
    ("_LOD1_Gls", FbxKind::Lod1Glass),
    ("_LOD1_Gra", FbxKind::Lod1Grass),
    ("_LOD1_Wat", FbxKind::Lod1Water),
    ("_LOD2_Win", FbxKind::Lod2Window),
    ("_LOD2_Wio", FbxKind::Lod2WioWindow),
    ("_LOD2_Wim", FbxKind::Lod2WimWindow),
    ("_LOD2_Wif", FbxKind::Lod2WifWindow),
    ("_LOD2_Gls", FbxKind::Lod2Glass),
    ("_LOD2_Gra", FbxKind::Lod2Grass),
    ("_LOD2_Wat", FbxKind::Lod2Water),
    ("_LOD1", FbxKind::Lod1),
    ("_LOD2", FbxKind::Lod2),
    ("_Win", FbxKind::Window),
    ("_Wio", FbxKind::WioWindow),
    ("_Wim", FbxKind::WimWindow),
    ("_Wif", FbxKind::WifWindow),
    ("_Gls", FbxKind::Glass),
    ("_Gra", FbxKind::Grass),
    ("_Wat", FbxKind::Water),
];

#[derive(Debug, PartialEq, Eq)]
pub enum Naming {
    Valid,
    Noncanonical,
    Ambiguous,
    Unsupported,
}

fn terminal_token(stem: &str) -> bool {
    let Some((_, token)) = stem.rsplit_once('_') else {
        return false;
    };
    let token = token.to_ascii_lowercase();
    SUFFIXES
        .iter()
        .any(|(suffix, _)| suffix[1..].eq_ignore_ascii_case(&token))
        || token
            .strip_prefix("lod")
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

pub fn classify_fbx(stem: &str) -> (&str, FbxKind, Naming) {
    for (suffix, kind) in SUFFIXES {
        if let Some((base, ending)) = stem
            .len()
            .checked_sub(suffix.len())
            .and_then(|i| stem.split_at_checked(i))
        {
            if ending.eq_ignore_ascii_case(suffix) {
                let naming = if base.trim().is_empty() {
                    Naming::Unsupported
                } else if terminal_token(base) {
                    Naming::Ambiguous
                } else if ending != *suffix {
                    Naming::Noncanonical
                } else {
                    Naming::Valid
                };
                return (base, *kind, naming);
            }
        }
    }
    (
        stem,
        FbxKind::Main,
        if stem.trim().is_empty() || terminal_token(stem) {
            Naming::Unsupported
        } else {
            Naming::Valid
        },
    )
}

pub fn inspect_fbx(path: &Path, kind: FbxKind, decal: bool) -> (FbxFile, Vec<Issue>) {
    let mut issues = Vec::new();
    let path_string = path.to_string_lossy();

    let (material_names, mesh_names, parse_error) =
        match ufbx::load_file(&path_string, ufbx::LoadOpts::default()) {
            Ok(scene) => {
                let mut materials = Vec::new();
                let mut meshes = Vec::new();
                for node in scene
                    .meshes
                    .iter()
                    .flat_map(|mesh| mesh.element.instances.iter())
                {
                    validate_material_slots(path, kind, decal, node.materials.len(), &mut issues);
                    for material in node.materials.iter() {
                        let name = material.element.name.as_ref();
                        if !name.is_empty() && !materials.iter().any(|existing| existing == name) {
                            materials.push(name.to_owned());
                        }
                    }
                    let name = node.element.name.as_ref();
                    if !name.is_empty() {
                        meshes.push(name.to_owned());
                    }
                }
                (materials, meshes, None)
            }
            Err(error) => {
                let message = if error.info().is_empty() {
                    error.description.as_ref().to_owned()
                } else {
                    format!("{} ({})", error.description.as_ref(), error.info())
                };
                issues.push(Issue::error(
                    "fbxParseFailed",
                    format!("The FBX could not be read: {message}"),
                    path,
                ));
                (Vec::new(), Vec::new(), Some(message))
            }
        };

    if parse_error.is_none() {
        validate_mesh_name(path, &mesh_names, &mut issues);
    }

    (
        FbxFile {
            path: path.to_path_buf(),
            kind,
            material_names,
            mesh_names,
            parse_error,
        },
        issues,
    )
}

fn validate_material_slots(
    path: &Path,
    kind: FbxKind,
    decal: bool,
    count: usize,
    issues: &mut Vec<Issue>,
) {
    let expected = usize::from(!decal && matches!(kind, FbxKind::Main | FbxKind::Lod1));
    if count != expected {
        issues.push(Issue::warning(
            "fbxMaterialSlotCount",
            format!(
                "Found {count} material slots; expected {expected}. {} and re-export the FBX.",
                if expected == 0 {
                    "Remove all material slots"
                } else {
                    "Assign one material to the mesh"
                }
            ),
            path,
        ));
    }
}

fn validate_mesh_name(path: &Path, mesh_names: &[String], issues: &mut Vec<Issue>) {
    let expected_stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();

    if mesh_names.len() != 1 || mesh_names.first() != Some(&expected_stem) {
        issues.push(Issue::warning(
            "meshNameMismatch",
            format!(
                "CS2 requires the FBX filename and its single mesh object name to match. Expected “{expected_stem}”; found: {}.",
                if mesh_names.is_empty() {
                    "none".into()
                } else {
                    mesh_names.join(", ")
                }
            ),
            path,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires the shared MW Batch Export Addon fixture"]
    fn classifies_shared_canonical_mesh_names() {
        let fixture_path = std::env::var_os("CS2_MESH_NAMING_FIXTURE")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../../MW Batch Export Addon/tests/cs2_mesh_names.json")
            });
        let json = std::fs::read_to_string(&fixture_path)
            .unwrap_or_else(|err| panic!("Cannot read {}: {err}", fixture_path.display()));
        let fixture: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(fixture["schema_version"].as_u64(), Some(1));
        let mut checked = 0;
        for case in fixture["cases"].as_array().unwrap() {
            // Repair suggestions for rejected inputs are not accepted exports.
            let input = case["input"]
                .as_str()
                .unwrap()
                .strip_suffix(".fbx")
                .unwrap();
            let (_, _, naming) = classify_fbx(input);
            assert_eq!(
                matches!(naming, Naming::Valid | Naming::Noncanonical),
                case["valid"].as_bool().unwrap(),
                "{input}"
            );
            if !case["valid"].as_bool().unwrap() {
                continue;
            }
            let filename = case["canonical"].as_str().unwrap();
            let stem = filename.strip_suffix(".fbx").unwrap();
            let (asset, kind, _) = classify_fbx(stem);
            assert_eq!(asset, case["asset"].as_str().unwrap(), "{filename}");
            let shader = match case["shader"].as_str() {
                None if case["shader"].is_null() => "",
                Some("Win") => "Window",
                Some("Wif") => "WifWindow",
                Some("Wio") => "WioWindow",
                Some("Wim") => "WimWindow",
                Some("Gls") => "Glass",
                Some("Gra") => "Grass",
                Some("Wat") => "Water",
                other => panic!("Unexpected shader {other:?} in {filename}"),
            };
            let expected_kind = match case["lod"].as_u64().unwrap() {
                0 if shader.is_empty() => "main".to_owned(),
                0 => format!("{}{}", shader[..1].to_ascii_lowercase(), &shader[1..]),
                lod @ (1 | 2) => format!("lod{lod}{shader}"),
                other => panic!("Unexpected LOD {other} in {filename}"),
            };
            assert_eq!(
                serde_json::to_value(kind).unwrap(),
                expected_kind,
                "{filename}"
            );
            checked += 1;
        }
        assert!(checked > 0, "Fixture has no valid exports");
    }

    #[test]
    fn distinguishes_noncanonical_ambiguous_and_unsupported_names() {
        assert_eq!(
            classify_fbx("House_lod1_wIF"),
            ("House", FbxKind::Lod1WifWindow, Naming::Noncanonical)
        );
        for stem in ["House_Wif_LOD1", "House_Win_Wio", "House_LOD1_LOD2"] {
            assert_eq!(classify_fbx(stem).2, Naming::Ambiguous);
        }
        for stem in ["House_LOD3", "_LOD1_Wif"] {
            assert_eq!(classify_fbx(stem).2, Naming::Unsupported);
        }
    }

    #[test]
    fn validates_material_slots_by_mesh_role() {
        for (kind, decal, count, invalid) in [
            (FbxKind::Main, false, 1, false),
            (FbxKind::Main, false, 0, true),
            (FbxKind::Main, false, 2, true),
            (FbxKind::Main, true, 0, false),
            (FbxKind::Lod2, false, 1, true),
            (FbxKind::Window, false, 1, true),
            (FbxKind::WifWindow, false, 1, true),
            (FbxKind::WioWindow, false, 1, true),
            (FbxKind::Lod2WioWindow, false, 0, false),
        ] {
            let mut issues = Vec::new();
            validate_material_slots(Path::new("House.fbx"), kind, decal, count, &mut issues);
            assert_eq!(!issues.is_empty(), invalid, "{kind:?}");
        }
    }

    #[test]
    fn classifies_longer_lod_submesh_suffixes_first() {
        assert_eq!(
            classify_fbx("House_LOD2_Win"),
            ("House", FbxKind::Lod2Window, Naming::Valid)
        );
        assert_eq!(
            classify_fbx("House_LOD1"),
            ("House", FbxKind::Lod1, Naming::Valid)
        );
        assert_eq!(
            classify_fbx("House"),
            ("House", FbxKind::Main, Naming::Valid)
        );
    }

    #[test]
    fn classifies_wio_wim_and_wif_as_window_submeshes() {
        assert_eq!(
            classify_fbx("House_Wio"),
            ("House", FbxKind::WioWindow, Naming::Valid)
        );
        assert_eq!(
            classify_fbx("House_LOD1_Wio"),
            ("House", FbxKind::Lod1WioWindow, Naming::Valid)
        );
        assert_eq!(
            classify_fbx("House_LOD2_Wio"),
            ("House", FbxKind::Lod2WioWindow, Naming::Valid)
        );
        assert_eq!(
            classify_fbx("House_Wim"),
            ("House", FbxKind::WimWindow, Naming::Valid)
        );
        assert_eq!(
            classify_fbx("House_Wif"),
            ("House", FbxKind::WifWindow, Naming::Valid)
        );
        assert_eq!(
            classify_fbx("House_LOD1_Wif"),
            ("House", FbxKind::Lod1WifWindow, Naming::Valid)
        );
        assert_eq!(
            classify_fbx("House_LOD2_Wif"),
            ("House", FbxKind::Lod2WifWindow, Naming::Valid)
        );

        // Window submeshes are displayed with their parent asset, but only the
        // material LOD meshes drive shared-texture settings entries.
        assert!(!FbxKind::Lod1WioWindow.is_lod1());
        assert!(!FbxKind::Lod2WioWindow.is_lod2());
        assert!(!FbxKind::Lod1WimWindow.is_lod1());
        assert!(!FbxKind::Lod2WimWindow.is_lod2());
        assert!(!FbxKind::Lod1WifWindow.is_lod1());
        assert!(!FbxKind::Lod2WifWindow.is_lod2());
    }

    #[test]
    fn matching_mesh_object_name_passes_validation() {
        let mut issues = Vec::new();

        validate_mesh_name(
            Path::new("House_LOD1.fbx"),
            &["House_LOD1".into()],
            &mut issues,
        );

        assert!(issues.is_empty());
    }

    #[test]
    fn mismatched_mesh_object_name_reports_expected_and_actual_names() {
        let mut issues = Vec::new();

        validate_mesh_name(Path::new("House_LOD1.fbx"), &["House".into()], &mut issues);

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, "meshNameMismatch");
        assert!(issues[0].message.contains("Expected “House_LOD1”"));
        assert!(issues[0].message.contains("found: House"));
    }

    #[test]
    fn multiple_mesh_objects_report_a_warning_even_when_names_match() {
        let mut issues = Vec::new();

        validate_mesh_name(
            Path::new("House.fbx"),
            &["House".into(), "House".into()],
            &mut issues,
        );

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, "meshNameMismatch");
    }
}
