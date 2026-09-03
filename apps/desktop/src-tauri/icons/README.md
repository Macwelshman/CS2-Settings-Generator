# Application icons

## macOS

`CS2Settings.icon` is the canonical adaptive macOS icon. It contains one
non-glass artwork layer with three appearance-specific images:

- `CS2-Settings-Icon-Light.png` for the default/light appearance
- `CS2-Settings-Icon-Dark 2.png` for the dark appearance
- `CS2-Settings-Icon-Glass 2.png` for the tinted/glass appearance

The images switch by appearance; they must not be stacked as simultaneous
layers or given an additional gradient/glass treatment. The build scripts
compile the Icon Composer document into `Assets.car` and `CS2Settings.icns`.
They also copy the compiled fallback to `icon.icns` before Cargo runs because
Tauri uses that conventional file as the Dock icon override in development
builds.

## Windows

`icon.ico` uses the rounded Light artwork from the compiled macOS fallback so
the Windows executable, NSIS installer, and uninstaller show the same design.
The ICO contains 16, 24, 32, 48, 64, 128, and 256 pixel representations.

## Other platforms

`icon-master-1024.png` remains the master for the conventional Tauri icon set.
Regenerate that set from the repository root with:

```sh
cargo tauri icon apps/desktop/src-tauri/icons/icon-master-1024.png \
  --output apps/desktop/src-tauri/icons
```

Do not replace the macOS Icon Composer document with the conventional
cross-platform icon output. If the conventional icon set is regenerated, run a
macOS build afterward so `icon.icns` is restored from `CS2Settings.icon`.
