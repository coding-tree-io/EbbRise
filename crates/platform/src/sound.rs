//! Plays a WAV file from memory at a volume, with `PlaySoundW` (ADR 0001: no audio crate).

use std::sync::Mutex;

use windows::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_MEMORY, SND_NODEFAULT};
use windows::core::PCWSTR;

/// The sound that plays now; Windows reads it while it plays, so it must stay alive.
static PLAYING: Mutex<Vec<u8>> = Mutex::new(Vec::new());

const PCM: u16 = 1;

/// The WAV with every sample scaled by `volume` (0–1). 8- and 16-bit PCM only; other
/// formats come back unchanged. `None` when the bytes are not a WAV file.
pub fn with_volume(wav: &[u8], volume: f64) -> Option<Vec<u8>> {
    if wav.len() < 12 || &wav[..4] != b"RIFF" || &wav[8..12] != b"WAVE" {
        return None;
    }
    let mut result = wav.to_vec();
    let volume = volume.clamp(0.0, 1.0);
    let mut format = None;
    let mut at = 12;
    while at + 8 <= wav.len() {
        let id = &wav[at..at + 4];
        let size = u32::from_le_bytes(wav[at + 4..at + 8].try_into().ok()?) as usize;
        let body = at + 8..(at + 8 + size).min(wav.len());
        if id == b"fmt " && body.len() >= 16 {
            let field = |offset: usize| {
                u16::from_le_bytes([wav[body.start + offset], wav[body.start + offset + 1]])
            };
            format = Some((field(0), field(14)));
        } else if id == b"data" {
            if let Some((PCM, bits)) = format {
                scale(&mut result[body], bits, volume);
            }
            return Some(result);
        }
        at += 8 + size + size % 2;
    }
    Some(result)
}

/// Scales 24- and 16-bit samples, or 8-bit ones, which are unsigned around 128; other sizes stay.
fn scale(samples: &mut [u8], bits: u16, volume: f64) {
    match bits {
        24 => {
            for sample in samples.as_chunks_mut::<3>().0 {
                let value = i32::from_le_bytes([0, sample[0], sample[1], sample[2]]) >> 8;
                let scaled = (f64::from(value) * volume).round() as i32;
                sample.copy_from_slice(&scaled.to_le_bytes()[..3]);
            }
        }
        16 => {
            for sample in samples.as_chunks_mut::<2>().0 {
                let scaled = (f64::from(i16::from_le_bytes(*sample)) * volume).round() as i16;
                *sample = scaled.to_le_bytes();
            }
        }
        8 => {
            for sample in samples {
                *sample = (128.0 + (f64::from(*sample) - 128.0) * volume).round() as u8;
            }
        }
        _ => {}
    }
}

/// Plays `wav` without waiting; a new sound stops the one before.
pub fn play(wav: Vec<u8>) {
    let mut playing = PLAYING
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // SAFETY: a null sound stops playback, so the old buffer is free to replace.
    unsafe {
        let _ = PlaySoundW(PCWSTR::null(), None, SND_ASYNC);
    }
    *playing = wav;
    // SAFETY: the buffer stays in PLAYING until the next call has stopped playback.
    unsafe {
        let _ = PlaySoundW(
            PCWSTR(playing.as_ptr().cast()),
            None,
            SND_MEMORY | SND_ASYNC | SND_NODEFAULT,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav(bits: u16, samples: &[u8]) -> Vec<u8> {
        let mut bytes = b"RIFF\0\0\0\0WAVEfmt ".to_vec();
        bytes.extend(16u32.to_le_bytes());
        bytes.extend(PCM.to_le_bytes()); // format
        bytes.extend(1u16.to_le_bytes()); // channels
        bytes.extend(8000u32.to_le_bytes()); // sample rate
        bytes.extend(8000u32.to_le_bytes()); // byte rate
        bytes.extend((bits / 8).to_le_bytes()); // block align
        bytes.extend(bits.to_le_bytes());
        bytes.extend(b"data");
        bytes.extend((samples.len() as u32).to_le_bytes());
        bytes.extend(samples);
        bytes
    }

    #[test]
    fn volume_scales_pcm_samples_and_keeps_the_header() {
        let loud = wav(16, &[0x00, 0x40, 0x00, 0xC0]); // 16384, -16384
        let half = with_volume(&loud, 0.5).unwrap();
        assert_eq!(half[..half.len() - 4], loud[..loud.len() - 4]);
        assert_eq!(half[half.len() - 4..], [0x00, 0x20, 0x00, 0xE0]);
        assert_eq!(with_volume(&loud, 1.0).unwrap(), loud);
        let eight_bit = with_volume(&wav(8, &[255, 128, 0]), 0.5).unwrap();
        assert_eq!(eight_bit[eight_bit.len() - 3..], [192, 128, 64]);
        // 24-bit: 0x400000 and -0x400000 halve to 0x200000 and -0x200000.
        let deep = with_volume(&wav(24, &[0, 0, 0x40, 0, 0, 0xC0]), 0.5).unwrap();
        assert_eq!(deep[deep.len() - 6..], [0, 0, 0x20, 0, 0, 0xE0]);
        assert_eq!(with_volume(b"not a wav file", 0.5), None);
    }
}
