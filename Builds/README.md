# Builds

This is the single folder for local builds that can be opened or installed.

The current release is **0.1.5**, with project-wide main texture selection and
parent-folder LOD2 inheritance. Its installers and updater packages are stored
in **0.1.5/macOS** and **0.1.5/Windows**. The top-level app is refreshed from the
macOS release package. Older packages are retained for reference.

Download published packages from the
[v0.1.5 release](https://github.com/Macwelshman/CS2-Settings-Generator/releases/tag/v0.1.5).

- `CS2 Settings Generator.app` is the convenient local app copy. The Codex **Run**
  action and `./script/build_and_run.sh` both rebuild it here.
- `CS2 Settings Generator_<version>_aarch64.dmg` is the latest packaged macOS
  release build created by `./script/build_release.sh`.
- Windows `.exe` and `.msi` installers are copied here when
  `./script/build_release_windows.ps1` is run on Windows.

The GitHub manual packaging workflow also collects both platform artifacts into
a folder named `Builds` before making them available for download.

Cargo's internal compiler output remains under `target/` and can be ignored.
