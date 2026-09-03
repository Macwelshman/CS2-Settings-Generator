# CS2 Settings Generator

Cross-platform macOS and Windows utility for scanning a Cities: Skylines II
asset export folder, checking mesh names and texture relationships, and
generating the `settings.json` files required for shared textures and decals.

[Download the latest release](https://github.com/Macwelshman/CS2-Settings-Generator/releases/latest)
or read the detailed [User Guide](docs/USER_GUIDE.md).

The current source includes three additions not yet published as a new release:

- **Window files:** `_Wio` and `_Wim` opaque windows, plus `_Wif` frosted
  windows, including their LOD1 and LOD2 variants.
- **Decals:** per-asset **Standard asset / Decal** selection, optional normal
  opacity, and shared textures in the same `settings.json`.
- **Software updates:** launch-time and manual checks, verified downloads,
  restart and replacement recovery.

Use the latest successful `main` Windows packaging run on the
[builds page](https://github.com/Macwelshman/CS2-Settings-Generator/actions/workflows/ci.yml)
and download its `CS2-Settings-Generator-Windows` artifact (GitHub sign-in may
be required). Test builds still display **0.1.3** and are distinct from the
older published 0.1.3 binary. The earlier build `1c597cb` was reported working
in UTM, but did not include the additional window-file changes.

## Download and install

### macOS

1. Download the Apple Silicon `.dmg` from the latest release.
2. Open the disk image and drag **CS2 Settings Generator** into Applications.
3. Open the app from Applications.

The macOS build is ad-hoc signed rather than notarized. If macOS blocks the
first launch, Control-click the app, select **Open**, then confirm **Open**.

### Windows

1. Download the x64 setup `.exe` from the latest release.
2. Run the installer and follow its prompts.
3. Open **CS2 Settings Generator** from the Start menu.

An `.msi` is also provided for managed installation. The Windows packages are
currently unsigned, so Windows may show a security warning during installation.

## Usage

### 1. Prepare an export folder

Place every asset folder and any shared texture folders beneath one overall
export folder. The scan is recursive, so textures do not need to be stored in
the same folder as the FBX that uses them.

```text
My Export/
├── Main Building/
│   ├── Main Building.fbx
│   ├── Main Building_LOD1.fbx
│   └── Main Building_LOD2.fbx
└── Shared Building Textures/
    ├── Shared Building Textures_BaseColor.png
    ├── Shared Building Textures_MaskMap.png
    └── Shared Building Textures_Normal.png
```

### 2. Scan the export

Drag the overall export folder into the app, or select **Scan Export Folder…**.
The app discovers asset folders, FBX variants, local textures, and shared
texture providers.

### 3. Review the results

Select an asset to review its detected main mesh, LODs, texture mappings,
warnings, blocking errors, and proposed `settings.json` content.

- LOD1 always uses the main mesh's texture set.
- LOD2 can use an independent texture set.
- Local asset-named textures take priority over shared textures.
- If a shared texture match is ambiguous, choose the intended provider from
  **Main + LOD1 texture set**.
- Use **Apply to other assets using…** when other non-local assets use the same
  FBX material and should receive the same selection.

### 4. Generate the files

For decal assets, first change **Asset type** from **Standard asset** to
**Decal**. Review the **Decal texture set** and optionally enable **Override
normal opacity**. BaseColor, MaskMap and Normal are required. Decal import
settings and any shared-texture redirects are written into the same file;
standard assets in the same scan are unaffected.

Resolve any blocking errors, review the preview, then select **Generate Settings
Files**. Each generated `settings.json` is written into its corresponding asset
folder.

Existing settings files are preserved by default. Enable **Replace existing
settings files** only when you intentionally want to overwrite them. **Clear**
and **Rescan** never delete or modify FBX files or textures.

## Workflow summary

1. Drag or choose an overall export folder.
2. Scan asset folders recursively.
3. Review detected main, LOD, sub-mesh, material, and texture relationships.
4. Resolve warnings or ambiguous shared-texture matches with the per-asset
   **Main + LOD1 texture set** selector. A selection can be applied to other
   non-local assets using the same FBX material.
5. Choose **Decal** for decal assets and review any normal-opacity override.
6. Preview and generate each asset folder's `settings.json`.

## Software updates

The updater checks for a newer stable version when the app opens. Use
**Check for Updates…** in the toolbar, or the application menu on Mac, to check
manually. An available update offers **Update Now**, **View Release**, and
**Later**.

Downloads are verified before installation. Updating restarts the app, so
generate any pending settings first: the current scan and ungenerated choices
are not retained. Export files are not changed by the updater.

Older versions need one manual installation of an updater-enabled build.
Automatic installation also requires a compatible update ZIP on the newer
release. If none is available, use **View Release** for the normal installer.
See the [User Guide](docs/USER_GUIDE.md#software-updates) for troubleshooting
and [update packaging notes](docs/SOFTWARE_UPDATES.md) for maintainers.

## Core generation rules

- Every folder containing a main FBX receives a `settings.json`.
- LOD1 always shares the main mesh's texture set.
- Textures already present under the required local filename need no redirect.
- Main or LOD2 textures stored elsewhere use portable relative paths.
- Existing settings files are skipped unless replacement is explicitly enabled.
- A generated shared-texture path must resolve to a real file.

## Validation

Recognised window sub-meshes include `_Win`, `_Wio`, `_Wim` and `_Wif`, with
`_LOD1` and `_LOD2` before the window suffix (for example,
`House_LOD1_Wio.fbx`). `_Wio` and `_Wim` are labelled opaque windows; `_Wif`
is labelled frosted windows. They are grouped with their parent asset, not
treated as separate main meshes, and do not create shared-texture redirects
of their own. The internal mesh-name check still applies.

The scanner will report import-readiness warnings alongside settings generation:

- Each FBX must contain one mesh object named exactly like the FBX file stem.
- A material may be shared by differently named meshes; its name is used to
  resolve the correct texture provider, not compared with the mesh filename.
- Material presence and material counts are not validated.
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

Run the Rust and updater-interface tests:

```sh
cargo test --workspace
node --test apps/desktop/tests/updates.test.cjs
```

Build the macOS release DMG:

Local and release Mac builds use the same adaptive Light/Dark/Glass icon
source and shared compilation script, including GitHub Actions packaging.
These builds require macOS 14 or later. The release is optimised; the local
test app is a debug build.

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
Choose `windows` to build the Windows installers and x64 update ZIP. Release
scripts also create SHA-256 sidecars for update ZIPs. Workflow artifacts are
test downloads, not published releases.

## Disclaimer

This is an independent community tool and is not affiliated with or endorsed
by Iceflake Studios or Paradox Interactive.

Cities: Skylines II and related names and trademarks belong to their respective
owners.
