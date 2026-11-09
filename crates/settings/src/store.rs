use std::fs;
use std::io::{self, ErrorKind, Write};
use std::path::Path;

use crate::Settings;

impl Settings {
    /// Loads the config at `own`. On first start, when `own` does not exist, imports the
    /// Stretchly config at `stretchly` and saves the result to `own`. A missing or broken
    /// Stretchly config imports the defaults.
    ///
    /// # Errors
    /// When `own` cannot be read or parsed, when `stretchly` exists but cannot be read, or
    /// when the first save fails. Nothing is saved then, so the next start tries again. A
    /// broken `own` stays as it is, so the user can repair it.
    pub fn load_or_import(own: &Path, stretchly: &Path) -> io::Result<Self> {
        match fs::read_to_string(own) {
            Ok(text) => Ok(Self::from_json(&text)?),
            Err(error) if error.kind() == ErrorKind::NotFound => {
                let imported = match fs::read_to_string(stretchly) {
                    Ok(text) => Self::from_json(&text).unwrap_or_default(),
                    Err(error) if error.kind() == ErrorKind::NotFound => Self::default(),
                    Err(error) => return Err(error),
                };
                imported.save(own)?;
                Ok(imported)
            }
            Err(error) => Err(error),
        }
    }

    /// Writes the settings to `path` as JSON and creates its folder when needed.
    ///
    /// The JSON goes to a temporary file first, which then replaces `path`. A crash leaves
    /// the old file or the new one, never half of one.
    ///
    /// # Errors
    /// When the folder or the file cannot be written.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(folder) = path.parent() {
            fs::create_dir_all(folder)?;
        }
        let temporary = path.with_extension("json.tmp");
        {
            let mut file = fs::File::create(&temporary)?;
            file.write_all(&serde_json::to_vec_pretty(self)?)?;
            file.sync_all()?;
        }
        fs::rename(&temporary, path)
    }
}
