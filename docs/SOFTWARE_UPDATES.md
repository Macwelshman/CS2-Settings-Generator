# Software updates and release packaging

The updater follows MW Combiner's public GitHub Releases / SHA-256 / staged replacement system. No credentials are included in the application.

The app checks the latest stable release at launch. Automatic check failures are quiet; **Check for Updates…** reports failures and up-to-date status. A newer release offers **Update Now**, **View Release**, and **Later**. Updating restarts the app and discards the current scan and ungenerated choices, not export files. The confirmation makes this explicit.

## Publishing compatible packages

Keep the workspace version, Tauri configuration version, and development bundle plist version aligned. The release tag must match the packaged version, for example `v0.1.5`. Do not relabel an older binary or mix package versions under one tag.

The platform release scripts collect normal installers plus these updater ZIPs:

- `CS2-Settings-Generator-<version>-macos-arm64.zip`
- `CS2-Settings-Generator-<version>-macos-x64.zip`
- `CS2-Settings-Generator-<version>-windows-x64.zip`
- `CS2-Settings-Generator-<version>-windows-arm64.zip`

Only publish architectures actually built and tested. Current CI builds Apple Silicon macOS and x64 Windows. The updater selects the running process architecture; it never substitutes another architecture. macOS ZIPs contain `CS2 Settings Generator.app`; Windows ZIPs contain only `cs2-settings-generator.exe`. Do not add guides, sidecars, or extra roots inside update ZIPs. Windows requires the WebView2 runtime provided by the initial installer.

Upload ZIPs as assets on a public stable release in `Macwelshman/CS2-Settings-Generator`. GitHub must return each asset's `sha256:` digest in its release API. The generated `.sha256` sidecars are for manual checking; the updater uses the API digest and refuses missing or mismatched digests. The newest release without a compatible ZIP remains viewable but cannot be installed automatically.

macOS packages are currently ad-hoc signed, not Developer ID signed or notarised. The updater verifies bundle identity, version, architecture and signature integrity. Windows currently ships unsigned binaries: the updater verifies SHA-256, PE product identity, version and architecture. This trusts the repository's release channel; it is not independent publisher signing.

## Replacement and recovery

Install in a writable location. A Mac app running under App Translocation must first be moved to Applications or another writable folder. Windows installations requiring administrator access should be updated using the normal installer; the updater does not elevate privileges.

Downloads are bounded, staged in a unique temporary directory and checked before shutdown. Archives with traversal paths, symbolic links or unexpected files are rejected. A separate copy of the current executable waits for the app to exit, moves the installed app/executable to a uniquely named sibling backup, and replaces it. A failed copy restores the backup. Only the app bundle on Mac or the app executable on Windows is replaced; neighbouring files are left alone.

If replacement fails, a dialog explains the failure, and `update-error.txt` remains in the temporary update directory. If restoration also fails, the message gives the retained backup location. A successful update removes downloaded and staged files. Windows may retain its small, locked helper executable in the system temporary directory.

The first release containing this utility must be installed manually once. In-app updating begins with the following compatible release. Local tests do not prove a complete live upgrade: verify replacement and rollback on both operating systems before publishing the bootstrap release.

## Validation commands

### Current validation status

The Windows x64 updater build at commit `1c597cb` passed the macOS and Windows
test jobs and Windows packaging in
[workflow run 33795645383](https://github.com/Macwelshman/CS2-Settings-Generator/actions/runs/33795645383).
The downloaded ZIP passed its integrity check and matched its SHA-256 sidecar;
the executable was verified as a Windows x64 GUI binary. The user subsequently
reported the build working in UTM. This is user-reported app testing, not
evidence of a complete newer-release installation or live rollback test.
That earlier build did not contain the subsequently integrated `_Wio`/`_Wim`/
`_Wif` window changes; use a newer `main` packaging artifact to test those.

The test package retains version `0.1.3`; it has not been published as a new
release. Give the eventual release a newer, consistently applied version so
existing updater-enabled copies can detect it.

### Repeatable checks

Run `cargo test --workspace` and `node --test apps/desktop/tests/updates.test.cjs`. On Mac, after building the app, set `CS2_UPDATE_TEST_APP` to its absolute bundle path and run `cargo test -p cs2-settings-desktop macos_package_round_trip -- --ignored` to check ZIP extraction, bundle metadata, architecture and signature together.

Before release, test the real update-and-restart flow from an older copy in a temporary installation folder on each supported platform. Check successful replacement, unwritable installation, missing/wrong digest, wrong version/identity, and recovery after a failed replacement. Do not test against the user's working installation.
