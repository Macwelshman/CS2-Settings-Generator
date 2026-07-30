use crate::fbx::{classify_fbx, inspect_fbx};
use crate::model::{AssetScan, FbxFile, FbxKind, Issue, ScanResult, TextureSet, TextureTier};
use crate::settings::build_settings_preview;
use crate::texture::collect_texture_sets;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug)]
pub struct ScanError {
    message: String,
}

impl ScanError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for ScanError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ScanError {}

pub fn scan_export_folder(root: &Path) -> Result<ScanResult, ScanError> {
    let root = fs::canonicalize(root).map_err(|error| {
        ScanError::new(format!(
            "Could not open export folder {}: {error}",
            root.display()
        ))
    })?;
    if !root.is_dir() {
        return Err(ScanError::new(format!(
            "The selected export path is not a folder: {}",
            root.display()
        )));
    }

    let mut fbx_paths = Vec::new();
    let mut png_paths = Vec::new();
    let mut global_issues = Vec::new();

    for entry in WalkDir::new(&root).follow_links(false) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                global_issues.push(Issue::warning(
                    "folderEntryUnreadable",
                    format!("A folder entry could not be read: {error}"),
                    &root,
                ));
                continue;
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }
        match entry
            .path()
            .extension()
            .map(|value| value.to_string_lossy().to_ascii_lowercase())
            .as_deref()
        {
            Some("fbx") => fbx_paths.push(entry.path().to_path_buf()),
            Some("png") => png_paths.push(entry.path().to_path_buf()),
            _ => {}
        }
    }
    fbx_paths.sort();
    png_paths.sort();

    let (texture_sets, texture_issues) = collect_texture_sets(&png_paths);
    global_issues.extend(texture_issues);

    let mut grouped: BTreeMap<(PathBuf, String), Vec<(PathBuf, FbxKind)>> = BTreeMap::new();
    for path in fbx_paths {
        let Some(stem) = path.file_stem().map(|value| value.to_string_lossy()) else {
            continue;
        };
        let (asset_name, kind) = classify_fbx(&stem);
        let folder = path.parent().unwrap_or(&root).to_path_buf();
        grouped
            .entry((folder, asset_name.to_owned()))
            .or_default()
            .push((path, kind));
    }

    let main_assets = grouped
        .iter()
        .filter(|(_, files)| files.iter().any(|(_, kind)| *kind == FbxKind::Main))
        .map(|(key, _)| key.clone())
        .collect::<BTreeSet<_>>();

    for ((folder, asset_name), files) in &grouped {
        if !files.iter().any(|(_, kind)| *kind == FbxKind::Main) {
            global_issues.push(Issue::warning(
                "orphanFbxVariant",
                format!("FBX variants for “{asset_name}” were found without a matching main FBX."),
                folder,
            ));
        }
    }

    let mut assets = Vec::new();
    for (folder, asset_name) in main_assets {
        let variants = grouped
            .remove(&(folder.clone(), asset_name.clone()))
            .unwrap_or_default();
        let mut issues = Vec::new();
        let mut files = Vec::<FbxFile>::new();
        for (path, kind) in variants {
            let (file, file_issues) = inspect_fbx(&path, kind);
            files.push(file);
            issues.extend(file_issues);
        }
        files.sort_by_key(|file| (file.kind, file.path.clone()));

        let has_lod1 = files.iter().any(|file| file.kind.is_lod1());
        let has_lod2 = files.iter().any(|file| file.kind.is_lod2());
        if !has_lod1 {
            issues.push(Issue::warning(
                "lod1Missing",
                "No LOD1 FBX was found. CS2 building assets require an LOD1 mesh.",
                &folder,
            ));
        }
        if !has_lod2 {
            issues.push(Issue::warning(
                "lod2Missing",
                "No LOD2 FBX was found. CS2 building assets require an LOD2 mesh.",
                &folder,
            ));
        }

        let main_material = files
            .iter()
            .find(|file| file.kind == FbxKind::Main)
            .and_then(|file| file.material_names.first().cloned());

        let main_texture_set = resolve_main_texture_set(
            &asset_name,
            &folder,
            main_material.as_deref(),
            &texture_sets,
            &mut issues,
        );
        let lod2_texture_set = resolve_lod2_texture_set(
            &asset_name,
            &folder,
            main_texture_set.as_ref(),
            &texture_sets,
            has_lod2,
            &mut issues,
        );

        let settings = build_settings_preview(
            &asset_name,
            &folder,
            has_lod1,
            has_lod2,
            main_texture_set.as_ref(),
            lod2_texture_set.as_ref(),
            &mut issues,
        );

        assets.push(AssetScan {
            name: asset_name,
            folder,
            files,
            main_texture_set,
            lod2_texture_set,
            settings,
            issues,
        });
    }
    assets.sort_by(|left, right| left.folder.cmp(&right.folder));

    Ok(ScanResult {
        root,
        assets,
        global_issues,
    })
}

fn resolve_main_texture_set(
    asset_name: &str,
    asset_folder: &Path,
    material_name: Option<&str>,
    texture_sets: &[TextureSet],
    issues: &mut Vec<Issue>,
) -> Option<TextureSet> {
    let local = matching_sets(texture_sets, asset_name, TextureTier::Main)
        .into_iter()
        .filter(|set| set.folder == asset_folder)
        .collect::<Vec<_>>();
    if local.len() == 1 {
        return Some(local[0].clone());
    }

    if let Some(material_name) = material_name {
        let material_base = material_name.strip_suffix("_Mtl").unwrap_or(material_name);
        let matches = matching_sets(texture_sets, material_base, TextureTier::Main);
        if matches.len() == 1 {
            return Some(matches[0].clone());
        }
        if matches.len() > 1 {
            issues.push(Issue::warning(
                "mainTextureSetAmbiguous",
                format!(
                    "Material “{material_name}” matches multiple texture sets; choose the intended set."
                ),
                asset_folder,
            ));
            return None;
        }
    }

    issues.push(Issue::warning(
        "mainTextureSetUnresolved",
        "No local texture set or unique material-name match was found for the main mesh.",
        asset_folder,
    ));
    None
}

fn resolve_lod2_texture_set(
    asset_name: &str,
    asset_folder: &Path,
    main_set: Option<&TextureSet>,
    texture_sets: &[TextureSet],
    has_lod2: bool,
    issues: &mut Vec<Issue>,
) -> Option<TextureSet> {
    if !has_lod2 {
        return None;
    }

    let local = matching_sets(texture_sets, asset_name, TextureTier::Lod2)
        .into_iter()
        .filter(|set| set.folder == asset_folder)
        .collect::<Vec<_>>();
    if local.len() == 1 {
        return Some(local[0].clone());
    }

    if let Some(main_set) = main_set {
        let matching_provider = matching_sets(texture_sets, &main_set.name, TextureTier::Lod2);
        if matching_provider.len() == 1 {
            return Some(matching_provider[0].clone());
        }
    }

    let all_lod2 = texture_sets
        .iter()
        .filter(|set| set.tier == TextureTier::Lod2)
        .cloned()
        .collect::<Vec<_>>();
    if all_lod2.len() == 1 {
        return Some(all_lod2[0].clone());
    }
    if all_lod2.len() > 1 {
        issues.push(Issue::warning(
            "lod2TextureSetAmbiguous",
            "Multiple LOD2 texture sets are available; choose the intended shared set.",
            asset_folder,
        ));
    }
    None
}

fn matching_sets(texture_sets: &[TextureSet], name: &str, tier: TextureTier) -> Vec<TextureSet> {
    texture_sets
        .iter()
        .filter(|set| set.name == name && set.tier == tier)
        .cloned()
        .collect()
}
