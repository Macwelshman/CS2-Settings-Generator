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

Every FBX should contain exactly one mesh object, and that object's name must
match the FBX filename without the `.fbx` extension. A mismatch produces an
import-readiness warning and does not modify the source FBX.

LOD2 is optional because smaller assets such as props and decals may not require
one. Its absence is not a warning; when an LOD2 FBX is present, its texture rules
are validated normally.

Decals do not require LOD1 or LOD2 meshes. Selecting **Decal** suppresses the
standard missing-LOD1 warning and requires a main BaseColor, MaskMap, and Normal
texture.

Material names are read only to help identify texture providers. The app does
not validate whether an FBX has a material, how many materials it has, or which
FBX variants contain them. A material name does not need to match the FBX or
mesh name, and multiple differently named meshes can correctly use a shared
material such as `Belfort Van Ghent` or `Belfort Van Ghent Details`.

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

## Automatic matching

1. Prefer a local texture set whose basename matches the asset.
2. Otherwise match the main FBX's single material name to a texture-set basename,
   allowing the documented `_Mtl` material suffix.
3. Prefer an LOD2 set belonging to the selected main texture provider.
4. If exactly one LOD2 texture set exists in the export folder, it can be chosen
   automatically.
5. Multiple candidates remain unresolved for user review.
