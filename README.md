# CS2 Settings Generator

Cross-platform macOS and Windows utility for scanning a Cities: Skylines II
asset export folder, validating its FBX and texture files, and generating the
`settings.json` files required for shared textures.

## Planned workflow

1. Drag or choose an overall export folder.
2. Scan asset folders recursively.
3. Review detected main, LOD, sub-mesh, material, and texture relationships.
4. Resolve warnings or ambiguous shared-texture matches.
5. Preview and generate each asset folder's `settings.json`.

## Core generation rules

- Every folder containing a main FBX receives a `settings.json`.
- LOD1 always shares the main mesh's texture set.
- Textures already present under the required local filename need no redirect.
- Main or LOD2 textures stored elsewhere use portable relative paths.
- Existing settings files are skipped unless replacement is explicitly enabled.
- A generated shared-texture path must resolve to a real file.

## Validation

The scanner will report import-readiness warnings alongside settings generation:

- Main and LOD1 meshes should contain exactly one material.
- LOD2 meshes should contain no materials.
- `_Win`, `_Wim`, `_Gls`, `_Gra`, and `_Wat` sub-meshes must contain no material.
- Texture names, formats, dimensions, and supported CS2 suffixes are checked.
- Ambiguous texture sets and unrecognised FBX suffixes require review.

The `Belfort Van Ghent` folder is retained locally as reference data and is
intentionally excluded from version control.
