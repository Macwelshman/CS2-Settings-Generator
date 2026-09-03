//! GitHub ZIP updates: the same release/digest contract as MW Combiner.
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

const REPO: &str = "https://github.com/Macwelshman/CS2-Settings-Generator";
const FEED: &str =
    "https://api.github.com/repos/Macwelshman/CS2-Settings-Generator/releases/latest";
const APP: &str = "CS2 Settings Generator.app";
const ID: &str = "com.macwelshman.cs2-settings-generator";
const EXE: &str = "cs2-settings-generator.exe";
static SELECTED: Mutex<Option<Release>> = Mutex::new(None);
static INSTALLING: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    digest: Option<String>,
}
#[derive(Clone, Deserialize)]
struct Release {
    tag_name: String,
    html_url: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<Asset>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    version: String,
    can_install: bool,
    message: String,
}

fn version(value: &str) -> Result<semver::Version, String> {
    let value = value.trim().trim_start_matches(['v', 'V']);
    let boundary = value.find(['-', '+']).unwrap_or(value.len());
    let mut core = value[..boundary].to_string();
    while core.matches('.').count() < 2 {
        core.push_str(".0");
    }
    let mut parsed = semver::Version::parse(&format!("{}{}", core, &value[boundary..]))
        .map_err(|_| "The release has an invalid version.")?;
    parsed.build = semver::BuildMetadata::EMPTY;
    Ok(parsed)
}
fn platform(os: &str, arch: &str) -> Result<&'static str, String> {
    match (os, arch) {
        ("macos", "aarch64") => Ok("macos-arm64"),
        ("macos", "x86_64") => Ok("macos-x64"),
        ("windows", "aarch64") => Ok("windows-arm64"),
        ("windows", "x86_64") => Ok("windows-x64"),
        _ => Err("This operating system or processor has no automatic update package.".into()),
    }
}
fn asset(release: &Release, target: &str) -> Result<Asset, String> {
    let name = format!(
        "CS2-Settings-Generator-{}-{target}.zip",
        version(&release.tag_name)?
    );
    let matches: Vec<_> = release.assets.iter().filter(|a| a.name == name).collect();
    if matches.len() != 1 {
        return Err(
            "This release has no matching update ZIP. Use View Release for a manual installer."
                .into(),
        );
    }
    let asset = matches[0];
    let digest = asset
        .digest
        .as_deref()
        .and_then(|d| d.strip_prefix("sha256:"));
    if !digest.is_some_and(|d| d.len() == 64 && d.bytes().all(|b| b.is_ascii_hexdigit())) {
        return Err("The update has no valid published SHA-256 digest.".into());
    }
    if !asset
        .browser_download_url
        .starts_with(&format!("{REPO}/releases/download/"))
    {
        return Err("The update download is not hosted by the application repository.".into());
    }
    Ok(asset.clone())
}
fn client() -> Result<Client, String> {
    Client::builder()
        .https_only(true)
        .user_agent("CS2-Settings-Generator-Updater")
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(180))
        .build()
        .map_err(|e| e.to_string())
}
fn check() -> Result<Option<UpdateInfo>, String> {
    let release: Release = client()?
        .get(FEED)
        .header("Accept", "application/vnd.github+json")
        .timeout(Duration::from_secs(20))
        .send()
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.json())
        .map_err(|e| format!("Could not check for updates: {e}"))?;
    let latest = version(&release.tag_name)?;
    if release.draft
        || release.prerelease
        || !latest.pre.is_empty()
        || latest <= version(env!("CARGO_PKG_VERSION"))?
    {
        *SELECTED.lock().map_err(|e| e.to_string())? = None;
        return Ok(None);
    }
    if !release
        .html_url
        .starts_with(&format!("{REPO}/releases/tag/"))
    {
        return Err("Invalid release page.".into());
    }
    let compatible =
        platform(std::env::consts::OS, std::env::consts::ARCH).and_then(|p| asset(&release, p));
    let info = UpdateInfo {
        version: latest.to_string(),
        can_install: compatible.is_ok(),
        message: compatible.err().unwrap_or_else(|| {
            "The app will restart after updating. Finish any pending settings changes first.".into()
        }),
    };
    *SELECTED.lock().map_err(|e| e.to_string())? = Some(release);
    Ok(Some(info))
}
#[tauri::command]
pub async fn check_for_updates() -> Result<Option<UpdateInfo>, String> {
    tauri::async_runtime::spawn_blocking(check)
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub fn view_update_release() -> Result<(), String> {
    let url = SELECTED
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .map(|r| r.html_url.clone())
        .unwrap_or_else(|| format!("{REPO}/releases/latest"));
    #[cfg(target_os = "macos")]
    Command::new("/usr/bin/open")
        .arg(url)
        .spawn()
        .map_err(|e| e.to_string())?;
    #[cfg(target_os = "windows")]
    Command::new("explorer.exe")
        .arg(url)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn run(command: &mut Command) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let output = command.output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "Update verification failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
fn extract(archive: &Path, destination: &Path) -> Result<(), String> {
    // Reject traversal, links, unexpected roots and oversized packages before extraction.
    let mut zip = zip::ZipArchive::new(fs::File::open(archive).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let mut total = 0u64;
    for index in 0..zip.len() {
        let entry = zip.by_index(index).map_err(|e| e.to_string())?;
        let path = entry
            .enclosed_name()
            .ok_or("Unsafe path in update archive.")?;
        let allowed = if cfg!(target_os = "macos") {
            path.starts_with(APP)
        } else {
            path == Path::new(EXE)
        };
        if !allowed || entry.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000) {
            return Err("Unexpected file or link in update archive.".into());
        }
        total = total
            .checked_add(entry.size())
            .ok_or("Update archive too large.")?;
        if total > 512 * 1024 * 1024 {
            return Err("Update archive too large.".into());
        }
    }
    drop(zip);
    #[cfg(target_os = "macos")]
    run(Command::new("/usr/bin/ditto")
        .args(["-x", "-k"])
        .arg(archive)
        .arg(destination))?;
    #[cfg(not(target_os = "macos"))]
    zip::ZipArchive::new(fs::File::open(archive).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?
        .extract(destination)
        .map_err(|e| e.to_string())?;
    Ok(())
}
fn installed_path() -> Result<PathBuf, String> {
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    #[cfg(target_os = "macos")]
    {
        let bundle = executable
            .parent()
            .and_then(Path::parent)
            .and_then(Path::parent)
            .ok_or("Not running from an app bundle.")?;
        if bundle.extension().is_none_or(|e| e != "app")
            || bundle.to_string_lossy().contains("/AppTranslocation/")
        {
            return Err(
                "Move the app to Applications or another writable folder before updating.".into(),
            );
        }
        Ok(bundle.to_owned())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(executable)
    }
}
fn verify(staged: &Path, expected: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let info = plist::Value::from_file(staged.join("Contents/Info.plist"))
            .map_err(|e| e.to_string())?;
        let info = info
            .as_dictionary()
            .ok_or("Invalid application metadata.")?;
        if info
            .get("CFBundleIdentifier")
            .and_then(plist::Value::as_string)
            != Some(ID)
            || version(
                info.get("CFBundleShortVersionString")
                    .and_then(plist::Value::as_string)
                    .unwrap_or(""),
            )? != version(expected)?
        {
            return Err("The update is for a different application or version.".into());
        }
        run(Command::new("/usr/bin/codesign")
            .args(["--verify", "--deep", "--strict"])
            .arg(staged))?;
        let architecture = if std::env::consts::ARCH == "aarch64" {
            "arm64"
        } else {
            "x86_64"
        };
        run(Command::new("/usr/bin/lipo")
            .arg(staged.join("Contents/MacOS/cs2-settings-generator"))
            .args(["-verify_arch", architecture]))?;
    }
    #[cfg(target_os = "windows")]
    {
        // Read PE resources without executing the downloaded program. Paths are data, not script text.
        let data = run(Command::new("powershell.exe").args(["-NoProfile", "-NonInteractive", "-Command", "$ErrorActionPreference='Stop'; $v=[Diagnostics.FileVersionInfo]::GetVersionInfo($env:CS2_UPDATE_EXE); @{product=$v.ProductName;version=$v.ProductVersion} | ConvertTo-Json -Compress"]).env("CS2_UPDATE_EXE", staged))?;
        let info: serde_json::Value = serde_json::from_str(&data).map_err(|e| e.to_string())?;
        if info["product"] != "CS2 Settings Generator"
            || version(info["version"].as_str().unwrap_or(""))? != version(expected)?
        {
            return Err("The update is for a different application or version.".into());
        }
        let bytes = fs::read(staged).map_err(|e| e.to_string())?;
        let offset = bytes.get(0x3c..0x40).ok_or("Invalid PE file.")?;
        let offset = u32::from_le_bytes(offset.try_into().unwrap()) as usize;
        let machine = if std::env::consts::ARCH == "aarch64" {
            [0x64, 0xaa]
        } else {
            [0x64, 0x86]
        };
        if bytes.get(..2) != Some(b"MZ")
            || bytes.get(offset..offset + 4) != Some(b"PE\0\0")
            || bytes.get(offset + 4..offset + 6) != Some(&machine)
        {
            return Err("Incorrect update processor architecture.".into());
        }
    }
    Ok(())
}

#[derive(Serialize, Deserialize)]
struct Plan {
    pid: u32,
    source: PathBuf,
    destination: PathBuf,
    backup: PathBuf,
}
fn stage() -> Result<(), String> {
    if rfd::MessageDialog::new().set_title("Update and restart?")
        .set_description("Your export files will not be changed, but the current scan and any ungenerated settings choices will be cleared. Update now?")
        .set_buttons(rfd::MessageButtons::OkCancel).show() != rfd::MessageDialogResult::Ok {
        return Err("Update cancelled. Nothing was changed.".into());
    }
    let release = SELECTED
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or("Check for an update first.")?;
    let asset = asset(
        &release,
        platform(std::env::consts::OS, std::env::consts::ARCH)?,
    )?;
    let destination = installed_path()?;
    let parent = destination.parent().ok_or("Invalid installation folder.")?;
    let probe = tempfile::Builder::new()
        .prefix(".cs2-update-")
        .tempfile_in(parent)
        .map_err(|_| "Move the app to a folder you can write to before updating.")?;
    drop(probe);
    let root = tempfile::Builder::new()
        .prefix("cs2-settings-update-")
        .tempdir()
        .map_err(|e| e.to_string())?;
    let archive = root.path().join("update.zip");
    let mut response = client()?
        .get(&asset.browser_download_url)
        .send()
        .and_then(|r| r.error_for_status())
        .map_err(|e| e.to_string())?;
    let mut file = fs::File::create(&archive).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    let mut size = 0u64;
    loop {
        let read = response.read(&mut buffer).map_err(|e| e.to_string())?;
        if read == 0 {
            break;
        }
        size += read as u64;
        if size > 256 * 1024 * 1024 {
            return Err("Update download exceeds the safety limit.".into());
        }
        hash.update(&buffer[..read]);
        file.write_all(&buffer[..read]).map_err(|e| e.to_string())?;
    }
    if format!("sha256:{:x}", hash.finalize()) != asset.digest.unwrap().to_lowercase() {
        return Err(
            "The download did not match its published SHA-256 digest. Nothing was installed."
                .into(),
        );
    }
    drop(file);
    let staging = root.path().join("staging");
    fs::create_dir(&staging).map_err(|e| e.to_string())?;
    extract(&archive, &staging)?;
    let source = staging.join(if cfg!(target_os = "macos") { APP } else { EXE });
    verify(&source, &release.tag_name)?;
    let backup = parent.join(format!(
        ".cs2-update-backup-{}",
        root.path().file_name().unwrap().to_string_lossy()
    ));
    let plan = Plan {
        pid: std::process::id(),
        source,
        destination,
        backup,
    };
    let plan_path = root.path().join("plan.json");
    fs::write(
        &plan_path,
        serde_json::to_vec(&plan).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let helper = root.path().join(if cfg!(windows) {
        "update-helper.exe"
    } else {
        "update-helper"
    });
    fs::copy(std::env::current_exe().map_err(|e| e.to_string())?, &helper)
        .map_err(|e| e.to_string())?;
    Command::new(helper)
        .arg("--apply-update")
        .arg(plan_path)
        .spawn()
        .map_err(|e| e.to_string())?;
    // The helper owns this directory after launch. It retains recovery evidence on failure.
    let _ = root.keep();
    Ok(())
}
#[tauri::command]
pub async fn install_update(app: tauri::AppHandle) -> Result<(), String> {
    if INSTALLING.swap(true, Ordering::SeqCst) {
        return Err("An update is already in progress.".into());
    }
    let result = tauri::async_runtime::spawn_blocking(stage)
        .await
        .map_err(|e| e.to_string())
        .and_then(|r| r);
    if result.is_ok() {
        app.exit(0);
    }
    INSTALLING.store(false, Ordering::SeqCst);
    result
}

fn apply(plan: &Plan) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let mut stopped = false;
        for _ in 0..300 {
            if !Command::new("/bin/kill")
                .args(["-0", &plan.pid.to_string()])
                .output()
                .map_err(|e| e.to_string())?
                .status
                .success()
            {
                stopped = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        if !stopped {
            return Err("The app did not close. No files were replaced.".into());
        }
    }
    #[cfg(target_os = "windows")]
    run(Command::new("powershell.exe").args(["-NoProfile", "-NonInteractive", "-Command", "$ErrorActionPreference='Stop'; $p=Get-Process -Id $env:CS2_UPDATE_PID -ErrorAction SilentlyContinue; if ($p) { Wait-Process -InputObject $p -Timeout 30 }"]).env("CS2_UPDATE_PID", plan.pid.to_string()))?;
    replace_package(plan)
}
fn replace_package(plan: &Plan) -> Result<(), String> {
    if plan.backup.exists() {
        return Err("An update recovery backup already exists.".into());
    }
    fs::rename(&plan.destination, &plan.backup)
        .map_err(|e| format!("Could not back up the current app: {e}"))?;
    #[cfg(target_os = "macos")]
    let copied = run(Command::new("/usr/bin/ditto")
        .arg(&plan.source)
        .arg(&plan.destination))
    .map(|_| ());
    #[cfg(not(target_os = "macos"))]
    let copied = fs::copy(&plan.source, &plan.destination)
        .map(|_| ())
        .map_err(|e| e.to_string());
    if let Err(error) = copied {
        if plan.destination.exists() {
            remove_package(&plan.destination)?;
        }
        fs::rename(&plan.backup, &plan.destination)
            .map_err(|e| format!("Restore the backup at {}: {e}", plan.backup.display()))?;
        return Err(format!(
            "Update failed; the previous app was restored: {error}"
        ));
    }
    remove_package(&plan.backup)?;
    Ok(())
}
fn remove_package(path: &Path) -> Result<(), String> {
    if path.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
    .map_err(|e| e.to_string())
}
/// Runs before Tauri starts; no webview is created by the replacement helper.
pub fn run_helper() -> bool {
    let args: Vec<_> = std::env::args_os().collect();
    if args.get(1).is_none_or(|s| s != "--apply-update") {
        return false;
    }
    let result = (|| -> Result<(), String> {
        let path = PathBuf::from(args.get(2).ok_or("Missing update plan.")?);
        let plan: Plan = serde_json::from_slice(&fs::read(&path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        let outcome = apply(&plan);
        if let Err(error) = &outcome {
            let _ = fs::write(path.with_file_name("update-error.txt"), error);
            rfd::MessageDialog::new()
                .set_title("Software Update")
                .set_description(error)
                .set_level(rfd::MessageLevel::Error)
                .show();
        }
        #[cfg(target_os = "macos")]
        let _ = Command::new("/usr/bin/open").arg(&plan.destination).spawn();
        #[cfg(target_os = "windows")]
        let _ = Command::new(&plan.destination).spawn();
        if outcome.is_ok() {
            // Windows locks the helper itself; retain only that small file and its directory.
            if let Some(root) = path.parent() {
                let _ = fs::remove_dir_all(root.join("staging"));
                let _ = fs::remove_file(root.join("update.zip"));
                let _ = fs::remove_file(&path);
                #[cfg(target_os = "macos")]
                let _ = fs::remove_dir_all(root);
            }
        }
        outcome
    })();
    if let Err(error) = result {
        eprintln!("{error}");
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "requires CS2_UPDATE_TEST_APP pointing to a freshly built app"]
    fn macos_package_round_trip() {
        let app = PathBuf::from(
            std::env::var_os("CS2_UPDATE_TEST_APP").expect("Set CS2_UPDATE_TEST_APP"),
        );
        let root = tempfile::tempdir().unwrap();
        let archive = root.path().join("update.zip");
        run(Command::new("/usr/bin/ditto")
            .args(["-c", "-k", "--norsrc", "--keepParent"])
            .arg(app)
            .arg(&archive))
        .unwrap();
        let staging = root.path().join("staging");
        fs::create_dir(&staging).unwrap();
        extract(&archive, &staging).unwrap();
        verify(&staging.join(APP), env!("CARGO_PKG_VERSION")).unwrap();
    }
    #[test]
    fn replacement_preserves_neighbouring_files() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("new");
        let destination = root.path().join("installed");
        fs::write(&source, "new version").unwrap();
        fs::write(&destination, "old version").unwrap();
        fs::write(root.path().join("user.txt"), "user data").unwrap();
        let plan = Plan {
            pid: 0,
            source,
            destination: destination.clone(),
            backup: root.path().join("backup"),
        };
        replace_package(&plan).unwrap();
        assert_eq!(fs::read_to_string(destination).unwrap(), "new version");
        assert_eq!(
            fs::read_to_string(root.path().join("user.txt")).unwrap(),
            "user data"
        );
        assert!(!plan.backup.exists());
    }
    #[test]
    fn versions_are_numeric() {
        assert!(version("v0.1.10").unwrap() > version("0.1.9").unwrap());
        assert_eq!(version("1.2").unwrap(), version("1.2.0").unwrap());
        assert!(version("1.2.0-rc.1").unwrap() < version("1.2.0").unwrap());
        assert_eq!(version("1.2.0+build.4").unwrap(), version("1.2").unwrap());
        assert!(version("invalid").is_err());
    }
    #[test]
    fn exact_platform_and_integrity_contract() {
        let mut release: Release = serde_json::from_value(serde_json::json!({"tag_name":"v1.2.3","html_url":format!("{REPO}/releases/tag/v1.2.3"),"draft":false,"prerelease":false,"assets":[{"name":"CS2-Settings-Generator-1.2.3-windows-x64.zip","browser_download_url":format!("{REPO}/releases/download/v1.2.3/update.zip"),"digest":format!("sha256:{}", "a".repeat(64))}]})).unwrap();
        assert!(asset(&release, platform("windows", "x86_64").unwrap()).is_ok());
        assert!(asset(&release, platform("windows", "aarch64").unwrap()).is_err());
        assert!(platform("linux", "x86_64").is_err());
        release.assets[0].digest = None;
        assert!(asset(&release, "windows-x64").is_err());
    }
    #[test]
    fn failed_copy_restores_original_and_preserves_neighbours() {
        let root = tempfile::tempdir().unwrap();
        let destination = root.path().join("app");
        fs::write(&destination, "original").unwrap();
        let neighbour = root.path().join("user.txt");
        fs::write(&neighbour, "untouched").unwrap();
        let plan = Plan {
            pid: 0,
            source: root.path().join("missing"),
            destination: destination.clone(),
            backup: root.path().join("backup"),
        };
        assert!(replace_package(&plan).is_err());
        assert_eq!(fs::read_to_string(destination).unwrap(), "original");
        assert_eq!(fs::read_to_string(neighbour).unwrap(), "untouched");
        assert!(!plan.backup.exists());
    }
    #[test]
    fn unsafe_archives_are_rejected() {
        for name in ["../outside.txt", "unrelated.txt"] {
            let root = tempfile::tempdir().unwrap();
            let archive = root.path().join("update.zip");
            let mut writer = zip::ZipWriter::new(fs::File::create(&archive).unwrap());
            writer
                .start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(b"unsafe").unwrap();
            writer.finish().unwrap();
            assert!(extract(&archive, &root.path().join("staging")).is_err());
        }
    }
}
