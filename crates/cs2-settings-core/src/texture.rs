use crate::model::{Issue, TextureFile, TextureKind, TextureSet, TextureTier};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

const SIMPLE_SUFFIXES: &[(&str, TextureKind)] = &[
    ("BaseColor", TextureKind::BaseColor),
    ("ControlMask", TextureKind::ControlMask),
    ("MaskMap", TextureKind::MaskMap),
    ("Normal", TextureKind::Normal),
    ("Emissive", TextureKind::Emissive),
];

pub fn collect_texture_sets(paths: &[PathBuf]) -> (Vec<TextureSet>, Vec<Issue>) {
    let mut groups: BTreeMap<(PathBuf, String, TextureTier), Vec<TextureFile>> = BTreeMap::new();
    let mut issues = Vec::new();

    for path in paths {
        let Some(stem) = path.file_stem().map(|value| value.to_string_lossy()) else {
            continue;
        };
        let Some((name, tier, kind)) = classify_texture(&stem) else {
            continue;
        };

        let dimensions = read_png_dimensions(path);
        let (width, height) = match dimensions {
            Ok(value) => value,
            Err(message) => {
                issues.push(Issue::warning("pngHeaderInvalid", message, path));
                (None, None)
            }
        };

        if let (Some(width), Some(height)) = (width, height) {
            if width != height {
                issues.push(Issue::warning(
                    "textureNotSquare",
                    format!("CS2 textures must be square; found {width}×{height}."),
                    path,
                ));
            }
            if !matches!(width, 512 | 1024 | 2048 | 4096)
                || !matches!(height, 512 | 1024 | 2048 | 4096)
            {
                issues.push(Issue::warning(
                    "textureResolutionUnsupported",
                    format!(
                        "Texture resolution {width}×{height} is outside the supported 512–4096 power-of-two sizes."
                    ),
                    path,
                ));
            }
        }

        let folder = path.parent().unwrap_or_else(|| Path::new("")).to_path_buf();
        groups
            .entry((folder, name, tier))
            .or_default()
            .push(TextureFile {
                path: path.clone(),
                kind,
                width,
                height,
            });
    }

    let mut sets = groups
        .into_iter()
        .map(|((folder, name, tier), mut files)| {
            files.sort_by_key(|file| file.kind.sort_order());
            let known_sizes = files
                .iter()
                .filter_map(|file| file.width.zip(file.height))
                .collect::<Vec<_>>();
            if let Some(first) = known_sizes.first().copied() {
                if known_sizes.iter().any(|size| *size != first) {
                    issues.push(Issue::warning(
                        "textureDimensionsMismatch",
                        format!("Texture set “{name}” contains mismatched dimensions."),
                        &folder,
                    ));
                }
            }
            TextureSet {
                name,
                tier,
                folder,
                files,
            }
        })
        .collect::<Vec<_>>();
    sets.sort_by(|left, right| {
        (&left.name, left.tier, &left.folder).cmp(&(&right.name, right.tier, &right.folder))
    });
    (sets, issues)
}

fn classify_texture(stem: &str) -> Option<(String, TextureTier, TextureKind)> {
    for (suffix, kind) in SIMPLE_SUFFIXES {
        let marker = format!("_{suffix}");
        if let Some(prefix) = stem.strip_suffix(&marker) {
            if let Some(name) = prefix.strip_suffix("_LOD2") {
                return Some((name.to_owned(), TextureTier::Lod2, *kind));
            }
            return Some((prefix.to_owned(), TextureTier::Main, *kind));
        }
    }

    for index in 0..=3 {
        let emissive_id = format!("_EmissiveID{index}");
        if let Some(prefix) = stem.strip_suffix(&emissive_id) {
            if let Some(name) = prefix.strip_suffix("_LOD2") {
                return Some((
                    name.to_owned(),
                    TextureTier::Lod2,
                    TextureKind::EmissiveId(index),
                ));
            }
            return Some((
                prefix.to_owned(),
                TextureTier::Main,
                TextureKind::EmissiveId(index),
            ));
        }

        let emissive = format!("_Emissive{index}");
        if let Some(prefix) = stem.strip_suffix(&emissive) {
            if let Some(name) = prefix.strip_suffix("_LOD2") {
                return Some((
                    name.to_owned(),
                    TextureTier::Lod2,
                    TextureKind::EmissiveIndexed(index),
                ));
            }
            return Some((
                prefix.to_owned(),
                TextureTier::Main,
                TextureKind::EmissiveIndexed(index),
            ));
        }
    }
    None
}

fn read_png_dimensions(path: &Path) -> Result<(Option<u32>, Option<u32>), String> {
    let mut file = File::open(path)
        .map_err(|error| format!("Could not open PNG to inspect dimensions: {error}"))?;
    let mut header = [0_u8; 24];
    file.read_exact(&mut header)
        .map_err(|error| format!("Could not read PNG header: {error}"))?;
    if &header[0..8] != b"\x89PNG\r\n\x1a\n" || &header[12..16] != b"IHDR" {
        return Err(
            "The file has a .png extension but does not contain a valid PNG header.".into(),
        );
    }
    let width = u32::from_be_bytes(header[16..20].try_into().expect("four bytes"));
    let height = u32::from_be_bytes(header[20..24].try_into().expect("four bytes"));
    Ok((Some(width), Some(height)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_main_and_lod2_texture_names() {
        assert_eq!(
            classify_texture("House_BaseColor"),
            Some(("House".into(), TextureTier::Main, TextureKind::BaseColor))
        );
        assert_eq!(
            classify_texture("House_LOD2_EmissiveID3"),
            Some((
                "House".into(),
                TextureTier::Lod2,
                TextureKind::EmissiveId(3)
            ))
        );
    }
}
