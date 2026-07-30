use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut arguments = env::args_os();
    let program = arguments
        .next()
        .and_then(|value| PathBuf::from(value).file_name().map(|name| name.to_owned()))
        .unwrap_or_else(|| "cs2-settings".into());

    let Some(root) = arguments.next().map(PathBuf::from) else {
        eprintln!(
            "Usage: {} <export-folder>\n\nScans without writing settings files.",
            program.to_string_lossy()
        );
        return ExitCode::from(2);
    };

    if arguments.next().is_some() {
        eprintln!("Only one export folder may be supplied.");
        return ExitCode::from(2);
    }

    match cs2_settings_core::scan_export_folder(&root) {
        Ok(result) => match serde_json::to_string_pretty(&result) {
            Ok(json) => {
                println!("{json}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("Could not serialise scan result: {error}");
                ExitCode::FAILURE
            }
        },
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
