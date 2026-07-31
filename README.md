# CS2 Settings Generator

Cross-platform macOS and Windows utility for scanning a Cities: Skylines II
asset export folder, validating its FBX and texture files, and generating the
`settings.json` files required for shared textures.

[Download the latest release](https://github.com/Macwelshman/CS2-Settings-Generator/releases/latest)
or read the [User Guide](docs/USER_GUIDE.md).

## Workflow

1. Drag or choose an overall export folder.
2. Scan asset folders recursively.
3. Review detected main, LOD, sub-mesh, material, and texture relationships.
4. Resolve warnings or ambiguous shared-texture matches with the per-asset
   **Main + LOD1 texture set** selector. A selection can be applied to other
   non-local assets using the same FBX material.
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

- Each FBX must contain one mesh object named exactly like the FBX file stem.
- Main and LOD1 meshes should contain exactly one material.
- LOD2 is optional; when present, its mesh should contain no materials.
- `_Win`, `_Wim`, `_Gls`, `_Gra`, and `_Wat` sub-meshes must contain no material.
- A material may be shared by differently named meshes; its name is used to
  resolve the correct texture provider, not compared with the mesh filename.
- An explicit per-asset texture selection overrides automatic resolution.
  Material-wide application never replaces an asset's own local, asset-named
  texture set.
- Texture names, formats, dimensions, and supported CS2 suffixes are checked.
- Ambiguous texture sets and unrecognised FBX suffixes require review.

## Build and package

All local builds intended for testing or installation are collected in the
top-level [`Builds`](Builds) folder.

Build the latest test app without launching it:

```sh
./script/build_and_run.sh --build-only
```

Build and launch the latest test app:

```sh
./script/build_and_run.sh
```

Run all cross-platform core tests:

```sh
cargo test --workspace
```

Build the macOS release DMG:

```sh
./script/build_release.sh
```

On Windows, build the `.exe` and `.msi` installers into the same `Builds`
folder from PowerShell:

```powershell
.\script\build_release_windows.ps1
```

The GitHub Actions workflow tests every push and pull request on macOS and
Windows. Its manual `workflow_dispatch` action also creates downloadable
macOS and Windows artifacts collected from a `Builds` folder on each runner.
