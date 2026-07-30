# Settings generation contract

This document records the first implementation contract for CS2 Settings
Generator. It is based on the supplied completed Belfort export collection and
the Cities: Skylines II Wiki documentation verified for game version 1.5.2f1.

## Folder boundary

The user selects one overall export folder. The scanner searches recursively
inside that folder and never creates a shared-texture reference to a file outside
it.

Each folder containing an unsuffixed main `.fbx` is treated as an Asset Folder.
A preview is produced for `<Asset Folder>/settings.json`.

## FBX recognition

Recognised material meshes:

- `<Asset>.fbx`
- `<Asset>_LOD1.fbx`
- `<Asset>_LOD2.fbx`

Recognised shader sub-mesh suffixes:

- `_Win` and `_Wim`
- `_Gls`
- `_Gra`
- `_Wat`

The same suffixes are recognised after `_LOD1` and `_LOD2`.

Main and LOD meshes should contain exactly one material. Shader sub-meshes must
contain no materials. These checks produce warnings and do not by themselves
prevent settings generation.

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
- If the main texture set is local and named for the asset, main entries are not
  required, but LOD1 aliases are still generated.
- If the main texture set is external, both main and LOD1 aliases are generated.
- Local LOD2 textures with exact expected filenames need no aliases.
- External LOD2 textures are added using relative paths.
- A generated `sharedAssets` destination must resolve to an existing file.
- Existing settings files are identified during scanning and are never replaced
  without an explicit user option.

## Automatic matching

1. Prefer a local texture set whose basename matches the asset.
2. Otherwise match the main FBX's single material name to a texture-set basename,
   allowing the documented `_Mtl` material suffix.
3. Prefer an LOD2 set belonging to the selected main texture provider.
4. If exactly one LOD2 texture set exists in the export folder, it can be chosen
   automatically.
5. Multiple candidates remain unresolved for user review.

