# Settings generation contract

This document records the first implementation contract for CS2 Settings
Generator. It is based on the supplied completed Belfort export collection and
the Cities: Skylines II Wiki documentation verified for game version 1.5.2f1.

## Folder boundary

The user selects one overall export folder. The scanner searches recursively
inside that folder and never creates a shared-texture reference to a file outside
it.

Texture discovery includes every recognised PNG set inside the scan boundary,
including the overall export folder and intermediate parent folders without an
FBX. Select the overall project folder for CS2 bulk import. The user confirmed
that CS2 allows this top-level selection with textures in the root folder. The
generator writes relative paths, not texture copies.

Each folder containing an unsuffixed main `.fbx` is treated as an Asset Folder.
A preview is produced for `<Asset Folder>/settings.json`.

Each detected asset defaults to **Standard asset**. The user can change an
individual asset to **Decal** without changing the type of other assets in the
same export-folder scan.

## FBX recognition

Recognised material meshes:

- `<Asset>.fbx`
- `<Asset>_LOD1.fbx`
- `<Asset>_LOD2.fbx`

Recognised shader sub-mesh suffixes:

- `_Win`
- `_Wio` and `_Wim` (opaque windows)
- `_Wif` (frosted windows)
- `_Gls`
- `_Gra`
- `_Wat`

The same suffixes are recognised after `_LOD1` and `_LOD2`.

To verify canonical FBX names against the shared exporter fixture, run:

```sh
cargo test -p cs2-settings-core classifies_shared_canonical_mesh_names -- --ignored
```

The test reads `tests/cs2_mesh_names.json` from a sibling `MW Batch Export Addon`
checkout by default. Set `CS2_MESH_NAMING_FIXTURE` to the fixture's path for
other layouts. It checks the asset and mesh kind of each valid canonical export.

Every FBX should contain exactly one mesh object, and that object's name must
match the FBX filename without the `.fbx` extension. A mismatch produces an
import-readiness warning and does not modify the source FBX.

LOD2 is optional because smaller assets such as props and decals may not require
one. Its absence is not a warning; when an LOD2 FBX is present, its texture rules
are validated normally.

Decals do not require LOD1 or LOD2 meshes. Selecting **Decal** suppresses the
standard missing-LOD1 warning and requires a main BaseColor, MaskMap, and Normal
texture.

Import Readiness checks material slots attached to mesh nodes. Standard main
and LOD1 meshes expect one slot; LOD2, shader submeshes, and decals expect
zero. Slot-count mismatches warn without preventing settings generation.
A material name may still be shared across different meshes.

Canonical suffixes match MW CS2 Batch Exporter. Case variants are accepted
with a warning. Repeated, conflicting, or reordered terminal role suffixes
are ambiguous and excluded; unsupported terminal LOD numbers and empty
asset names are also excluded. Variants must share the exact asset stem and
folder. Duplicate roles or multiple main assets competing for one output
block settings generation in that folder.

## Texture recognition

The main and LOD2 texture sets can contain:

- `BaseColor`
- `ControlMask`
- `MaskMap`
- `Normal`
- `Emissive`
- paired `Emissive0`–`Emissive3` and `EmissiveID0`–`EmissiveID3`

Texture files must be PNG. Import-readiness validation checks square,
power-of-two dimensions from 512 through 4096 and matching dimensions within a
set.

## Shared-texture rules

- LOD1 always shares the main mesh's texture set.
- Match the main FBX material name to a texture-set basename across the
  project. Folder names and FBX filenames do not override this match.
- Missing or ambiguous material matches block generation until resolved.
  An unrelated local set or the only available set is never a fallback.
- Local textures named for the asset require no main aliases; LOD1 aliases
  are always generated when an LOD1 mesh exists.
- If the main texture set is external, both main and LOD1 aliases are generated.
- Local LOD2 textures with exact expected filenames need no aliases.
- External LOD2 textures are added using relative paths.
- A generated `sharedAssets` destination must resolve to an existing file.
- Existing settings files are identified during scanning and are never replaced
  without an explicit user option.

## Decal import settings

For an asset explicitly selected as **Decal**, the generated file adds:

```json
"SurfacePostProcessor": {
  "materialTemplate": "DefaultDecal"
}
```

The optional normal-opacity override adds `floatProperties._NormalOpacity` with
a value from 0 through 1. This decal section and `sharedAssets` are written into
the same `settings.json`, so local and shared decal texture sets follow the same
path-resolution rules as other assets.

## Texture selection and automatic matching

1. An asset's explicit Main + LOD1 selection takes priority over the project
   main texture selection. With neither selection, match the main FBX material
   name to a texture-set basename, allowing the `_Mtl` suffix.
2. Missing or multiple automatic main matches require review. A missing explicit
   selection blocks generation instead of silently changing sources. LOD1 always
   inherits the resolved main set.
3. An explicit LOD2 selection takes priority. Otherwise prefer an exact local
   LOD2 set, then the first ancestor folder within the scan containing LOD2 sets.
   One set is inherited; multiple sets block generation until manually resolved.
4. If no ancestor supplies LOD2, retain the existing provider fallback: use a
   matching LOD2 set in the selected main provider folder, then a sole LOD2 set
   in an importable asset folder. Multiple candidates block generation. Loose
   LOD2 sets in other asset groups' parent folders are not global fallbacks.
5. All explicit selections must refer to discovered sets of the correct tier
   inside the scan boundary. Main and LOD2 selections are independent.
6. Desktop selections are saved per canonical root, in a versioned local
   profile. The scan root is absolute; selection folders inside it are relative.
   The scan always includes subfolders and LOD2 defaults to automatic
   inheritance unless an override is saved. The profile records the project
   provider, per-asset main and LOD2 overrides, asset type and decal opacity.
   Missing texture providers remain selected and require review. Missing or
   renamed assets retain their choices without transferring them to a different
   mesh. The replacement checkbox is reset when reopening a project.

## Comparing existing settings

Changes Only compares parsed JSON, ignoring formatting and object key order.
It shows additions, removals and old/new values for modifications. Paths are
JSON Pointers, and arrays are compared as whole values. Invalid or unreadable
existing JSON is reported. Desktop generation checks that its current scan
still matches the preview before writing.
