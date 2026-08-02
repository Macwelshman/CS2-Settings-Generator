# Builds

This is the single folder for local builds that can be opened or installed.

- `CS2 Settings Generator.app` is the latest local test build. The Codex **Run**
  action and `./script/build_and_run.sh` both rebuild it here.
- `CS2 Settings Generator_<version>_aarch64.dmg` is the latest packaged macOS
  release build created by `./script/build_release.sh`.
- Windows `.exe` and `.msi` installers are copied here when
  `./script/build_release_windows.ps1` is run on Windows.

The GitHub manual packaging workflow also collects both platform artifacts into
a folder named `Builds` before making them available for download.

Cargo's internal compiler output remains under `target/` and can be ignored.
