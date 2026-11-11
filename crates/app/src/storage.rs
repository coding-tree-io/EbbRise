//! The data folder `%APPDATA%\EbbRise`: config, sounds and logs.

use std::path::{Path, PathBuf};

use settings::Settings;
use tracing::{error, info, warn};

use crate::logging;

/// Starts the log file and creates the audio folder, whose path it returns.
pub(crate) fn prepare_app_folder() -> Option<PathBuf> {
    let folder = app_folder()?;
    logging::start(&log_path(&folder));
    let audio = folder.join("audio");
    if let Err(error) = std::fs::create_dir_all(&audio) {
        warn!(%error, "audio folder not created");
    }
    Some(audio)
}

/// `%APPDATA%\EbbRise`, where the config, the sounds and the logs live.
pub(crate) fn app_folder() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|app_data| PathBuf::from(app_data).join("EbbRise"))
}

pub(crate) fn log_path(app_folder: &Path) -> PathBuf {
    app_folder.join("logs").join("main.log")
}

pub(crate) fn load_settings(folder: &Path) -> std::io::Result<(Settings, bool)> {
    let own = folder.join("config.json");
    let stretchly = folder.with_file_name("Stretchly").join("config.json");
    match Settings::load_or_import(&own, &stretchly) {
        Ok(settings) => {
            info!(path = %own.display(), "settings loaded");
            Ok((settings, true))
        }
        Err(error) => {
            error!(path = %own.display(), %error, "settings unreadable, using defaults");
            if own.try_exists()? {
                let backup = folder.join("config.unreadable.json");
                std::fs::copy(&own, &backup)?;
                warn!(path = %backup.display(), "unreadable settings backed up");
            }
            Ok((Settings::default(), false))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_config_is_backed_up_before_defaults_are_used() {
        let folder = std::env::temp_dir().join(format!("ebbrise-config-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("config.json"), "{broken").unwrap();
        let (_, loaded) = load_settings(&folder).unwrap();
        assert!(!loaded);
        assert_eq!(
            std::fs::read_to_string(folder.join("config.unreadable.json")).unwrap(),
            "{broken"
        );
        std::fs::remove_dir_all(folder).unwrap();
    }
}
