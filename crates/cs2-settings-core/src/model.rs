use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub root: PathBuf,
    pub assets: Vec<AssetScan>,
    pub texture_sets: Vec<TextureSet>,
    pub global_issues: Vec<Issue>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextureSetOverride {
    pub asset_folder: PathBuf,
    pub texture_set_folder: PathBuf,
    pub texture_set_name: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationReport {
    pub root: PathBuf,
    pub items: Vec<GenerationItem>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationItem {
    pub asset_name: String,
    pub output_path: PathBuf,
    pub action: GenerationAction,
    pub message: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum GenerationAction {
    Generated,
    Replaced,
    SkippedExisting,
    SkippedInvalid,
    Failed,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetScan {
    pub name: String,
    pub folder: PathBuf,
    pub files: Vec<FbxFile>,
    pub main_texture_set: Option<TextureSet>,
    pub lod2_texture_set: Option<TextureSet>,
    pub settings: SettingsPreview,
    pub issues: Vec<Issue>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FbxFile {
    pub path: PathBuf,
    pub kind: FbxKind,
    pub material_names: Vec<String>,
    pub mesh_names: Vec<String>,
    pub parse_error: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FbxKind {
    Main,
    Lod1,
    Lod2,
    Window,
    MilkyWindow,
    Glass,
    Grass,
    Water,
    Lod1Window,
    Lod1MilkyWindow,
    Lod1Glass,
    Lod1Grass,
    Lod1Water,
    Lod2Window,
    Lod2MilkyWindow,
    Lod2Glass,
    Lod2Grass,
    Lod2Water,
}

impl FbxKind {
    pub fn requires_one_material(self) -> bool {
        matches!(self, Self::Main | Self::Lod1)
    }

    pub fn requires_no_material(self) -> bool {
        !self.requires_one_material()
    }

    pub fn is_lod1(self) -> bool {
        matches!(self, Self::Lod1)
    }

    pub fn is_lod2(self) -> bool {
        matches!(self, Self::Lod2)
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextureSet {
    pub name: String,
    pub tier: TextureTier,
    pub folder: PathBuf,
    pub files: Vec<TextureFile>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TextureTier {
    Main,
    Lod2,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextureFile {
    pub path: PathBuf,
    pub kind: TextureKind,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TextureKind {
    BaseColor,
    ControlMask,
    MaskMap,
    Normal,
    Emissive,
    EmissiveIndexed(u8),
    EmissiveId(u8),
}

impl TextureKind {
    pub fn suffix(self) -> String {
        match self {
            Self::BaseColor => "BaseColor".into(),
            Self::ControlMask => "ControlMask".into(),
            Self::MaskMap => "MaskMap".into(),
            Self::Normal => "Normal".into(),
            Self::Emissive => "Emissive".into(),
            Self::EmissiveIndexed(index) => format!("Emissive{index}"),
            Self::EmissiveId(index) => format!("EmissiveID{index}"),
        }
    }

    pub fn sort_order(self) -> (u8, u8) {
        match self {
            Self::BaseColor => (0, 0),
            Self::ControlMask => (1, 0),
            Self::Emissive => (2, 0),
            Self::EmissiveIndexed(index) => (3, index),
            Self::EmissiveId(index) => (4, index),
            Self::MaskMap => (5, 0),
            Self::Normal => (6, 0),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPreview {
    pub output_path: PathBuf,
    pub entries: Vec<SharedAssetEntry>,
    pub json: String,
    pub existing_file: bool,
    pub can_generate: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedAssetEntry {
    pub shared_to: String,
    pub shared_from: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub severity: IssueSeverity,
    pub code: String,
    pub message: String,
    pub path: Option<PathBuf>,
}

impl Issue {
    pub fn warning(
        code: impl Into<String>,
        message: impl Into<String>,
        path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            severity: IssueSeverity::Warning,
            code: code.into(),
            message: message.into(),
            path: Some(path.into()),
        }
    }

    pub fn error(
        code: impl Into<String>,
        message: impl Into<String>,
        path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            severity: IssueSeverity::Error,
            code: code.into(),
            message: message.into(),
            path: Some(path.into()),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum IssueSeverity {
    Warning,
    Error,
}
