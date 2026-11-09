//! Break sounds: three built into the exe (credits in `audio/CREDITS.md`), and the user's own
//! WAV files in the audio folder. A user file with a built-in name replaces that sound, as in
//! upstream.

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use tracing::warn;

/// The built-in sounds, by the names Stretchly's settings use.
const BUILT_IN: [(&str, &[u8]); 3] = [
    (
        "crystal-glass",
        include_bytes!("../audio/crystal-glass.wav"),
    ),
    ("tic-toc", include_bytes!("../audio/tic-toc.wav")),
    ("wind-chime", include_bytes!("../audio/wind-chime.wav")),
];

/// The WAV bytes for `name`: the user's file first, then the built-in sound.
pub(crate) fn wav(folder: Option<&Path>, name: &str) -> Option<Cow<'static, [u8]>> {
    if let Some(path) = folder.and_then(|folder| sound_file(folder, name)) {
        match std::fs::read(&path) {
            Ok(bytes) => return Some(Cow::Owned(bytes)),
            Err(error) => warn!(path = %path.display(), %error, "sound unreadable"),
        }
    }
    BUILT_IN
        .iter()
        .find(|(built_in, _)| *built_in == name)
        .map(|(_, bytes)| Cow::Borrowed(*bytes))
}

/// The built-in names and the names of the WAV files in `folder`, sorted, without repeats.
pub(crate) fn names(folder: Option<&Path>) -> Vec<String> {
    let mut names: Vec<String> = BUILT_IN
        .iter()
        .map(|(name, _)| (*name).to_owned())
        .collect();
    if let Some(entries) = folder.and_then(|folder| std::fs::read_dir(folder).ok()) {
        names.extend(
            entries
                .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
                .filter_map(|file| file.strip_suffix(".wav").map(str::to_owned))
                .filter(|name| folder.and_then(|f| sound_file(f, name)).is_some()),
        );
    }
    names.sort();
    names.dedup();
    names
}

/// `<name>.wav` in `folder`, when the name is a plain file name (upstream `audioResolver`)
/// and the file exists.
fn sound_file(folder: &Path, name: &str) -> Option<PathBuf> {
    let file = format!("{name}.wav");
    let plain = file
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
    let path = folder.join(file);
    (plain && path.is_file()).then_some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sound_files_are_plain_names_inside_the_audio_folder() {
        let folder = std::env::temp_dir().join(format!("ebbrise-audio-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("my-chime_2.wav"), b"RIFF").unwrap();
        assert_eq!(
            sound_file(&folder, "my-chime_2"),
            Some(folder.join("my-chime_2.wav"))
        );
        assert_eq!(sound_file(&folder, "missing"), None);
        assert_eq!(sound_file(&folder, r"..\my-chime_2"), None);
        assert_eq!(sound_file(&folder, "sub/my-chime_2"), None);
        assert_eq!(
            names(Some(&folder)),
            ["crystal-glass", "my-chime_2", "tic-toc", "wind-chime"]
        );
        // A user file replaces the built-in sound of the same name.
        std::fs::write(folder.join("tic-toc.wav"), b"RIFF").unwrap();
        assert_eq!(wav(Some(&folder), "tic-toc").as_deref(), Some(&b"RIFF"[..]));
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn built_in_sounds_are_wav_files() {
        for (name, _) in BUILT_IN {
            let bytes = wav(None, name).unwrap();
            assert!(
                platform::sound::with_volume(&bytes, 0.5).is_some(),
                "{name}"
            );
        }
    }
}
