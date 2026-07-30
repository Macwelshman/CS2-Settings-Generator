use crate::model::{FbxFile, FbxKind, Issue};
use std::path::Path;

const SUFFIXES: &[(&str, FbxKind)] = &[
    ("_LOD1_Win", FbxKind::Lod1Window),
    ("_LOD1_Wim", FbxKind::Lod1MilkyWindow),
    ("_LOD1_Gls", FbxKind::Lod1Glass),
    ("_LOD1_Gra", FbxKind::Lod1Grass),
    ("_LOD1_Wat", FbxKind::Lod1Water),
    ("_LOD2_Win", FbxKind::Lod2Window),
    ("_LOD2_Wim", FbxKind::Lod2MilkyWindow),
    ("_LOD2_Gls", FbxKind::Lod2Glass),
    ("_LOD2_Gra", FbxKind::Lod2Grass),
    ("_LOD2_Wat", FbxKind::Lod2Water),
    ("_LOD1", FbxKind::Lod1),
    ("_LOD2", FbxKind::Lod2),
    ("_Win", FbxKind::Window),
    ("_Wim", FbxKind::MilkyWindow),
    ("_Gls", FbxKind::Glass),
    ("_Gra", FbxKind::Grass),
    ("_Wat", FbxKind::Water),
];

pub fn classify_fbx(stem: &str) -> (&str, FbxKind) {
    for (suffix, kind) in SUFFIXES {
        if let Some(base) = stem.strip_suffix(suffix) {
            return (base, *kind);
        }
    }
    (stem, FbxKind::Main)
}

pub fn inspect_fbx(path: &Path, kind: FbxKind) -> (FbxFile, Vec<Issue>) {
    let mut issues = Vec::new();
    let path_string = path.to_string_lossy();

    let (material_names, mesh_names, parse_error) =
        match ufbx::load_file(&path_string, ufbx::LoadOpts::default()) {
            Ok(scene) => {
                let mut materials = Vec::new();
                for material in scene.materials.iter() {
                    let name = material.element.name.as_ref();
                    if !name.is_empty() && !materials.iter().any(|existing| existing == name) {
                        materials.push(name.to_owned());
                    }
                }
                let mut meshes = Vec::new();
                for node in scene
                    .meshes
                    .iter()
                    .flat_map(|mesh| mesh.element.instances.iter())
                {
                    let name = node.element.name.as_ref();
                    if !name.is_empty() && !meshes.iter().any(|existing| existing == name) {
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
        if kind.requires_one_material() {
            match material_names.len() {
                0 => issues.push(Issue::warning(
                    "materialMissing",
                    "Main and LOD1 meshes should contain exactly one material; none were found.",
                    path,
                )),
                1 => {}
                count => issues.push(Issue::warning(
                    "multipleMaterials",
                    format!(
                        "Main and LOD1 meshes support exactly one material; {count} were found: {}.",
                        material_names.join(", ")
                    ),
                    path,
                )),
            }
        } else if kind.requires_no_material() && !material_names.is_empty() {
            issues.push(Issue::warning(
                "materialNotAllowed",
                format!(
                    "LOD2 and shader sub-mesh FBXs must contain no materials; found: {}.",
                    material_names.join(", ")
                ),
                path,
            ));
        }

        let expected_stem = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default();
        if mesh_names.len() != 1 || mesh_names.first() != Some(&expected_stem) {
            issues.push(Issue::warning(
                "meshNameMismatch",
                format!(
                    "The FBX should contain one mesh object named “{expected_stem}”; found: {}.",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_longer_lod_submesh_suffixes_first() {
        assert_eq!(
            classify_fbx("House_LOD2_Win"),
            ("House", FbxKind::Lod2Window)
        );
        assert_eq!(classify_fbx("House_LOD1"), ("House", FbxKind::Lod1));
        assert_eq!(classify_fbx("House"), ("House", FbxKind::Main));
    }

    #[test]
    fn lod2_and_shader_meshes_require_no_materials() {
        assert!(FbxKind::Main.requires_one_material());
        assert!(FbxKind::Lod1.requires_one_material());
        assert!(FbxKind::Lod2.requires_no_material());
        assert!(FbxKind::Window.requires_no_material());
        assert!(FbxKind::Lod2Window.requires_no_material());
    }
}
