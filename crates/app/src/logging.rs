//! Log file in the style of upstream's electron-log: `main.log`, moved to `main.old.log` when
//! it passes 1 MB.

use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::Path;
use std::sync::Mutex;

const MAX_LOG_BYTES: u64 = 1024 * 1024;

/// Sends `tracing` events at info level and above to the file at `path`.
///
/// Without a log file the app still runs; the reason goes to stderr.
pub fn start(path: &Path) {
    match open(path) {
        Ok(file) => tracing_subscriber::fmt()
            .with_writer(Mutex::new(file))
            .with_ansi(false)
            .init(),
        Err(error) => eprintln!("no log file at {}: {error}", path.display()),
    }
}

/// Opens `path` to append; first moves a file larger than [`MAX_LOG_BYTES`] to `main.old.log`.
///
/// shortcut: checks the size only at start; a few lines per break keep a day far below 1 MB.
fn open(path: &Path) -> io::Result<File> {
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder)?;
    }
    if fs::metadata(path).is_ok_and(|file| file.len() > MAX_LOG_BYTES) {
        fs::rename(path, path.with_extension("old.log"))?;
    }
    OpenOptions::new().create(true).append(true).open(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_full_log_moves_to_the_old_log_and_a_small_one_grows() {
        let folder = std::env::temp_dir().join(format!("ebbrise-log-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        let log = folder.join("logs").join("main.log");

        drop(open(&log).unwrap());
        fs::write(&log, "small").unwrap();
        drop(open(&log).unwrap());
        assert_eq!(fs::read_to_string(&log).unwrap(), "small");

        fs::write(&log, vec![b'x'; MAX_LOG_BYTES as usize + 1]).unwrap();
        drop(open(&log).unwrap());
        assert_eq!(fs::metadata(&log).unwrap().len(), 0);
        let old = folder.join("logs").join("main.old.log");
        assert_eq!(fs::metadata(old).unwrap().len(), MAX_LOG_BYTES + 1);
        fs::remove_dir_all(folder).unwrap();
    }
}
