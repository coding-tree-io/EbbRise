//! Media transport commands on a dedicated worker, away from the UI thread.

use std::sync::mpsc::{self, Receiver, Sender};
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession as Session,
    GlobalSystemMediaTransportControlsSessionManager as Manager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
};
use windows::Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize};

/// Serializes media pause and resume requests on a worker thread.
pub struct MediaControl(Sender<bool>);

impl Default for MediaControl {
    fn default() -> Self {
        Self::new()
    }
}

impl MediaControl {
    /// Starts the worker.
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || run(receiver));
        Self(sender)
    }

    /// Queues a pause of sessions that are playing.
    pub fn pause(&self) {
        if let Err(error) = self.0.send(true) {
            tracing::warn!(%error, "media worker unavailable");
        }
    }

    /// Queues a resume of sessions this worker paused, if still paused.
    pub fn resume(&self) {
        if let Err(error) = self.0.send(false) {
            tracing::warn!(%error, "media worker unavailable");
        }
    }
}

fn run(receiver: Receiver<bool>) {
    // SAFETY: this worker initializes and uninitializes its own WinRT apartment.
    if let Err(error) = unsafe { RoInitialize(RO_INIT_MULTITHREADED) } {
        tracing::warn!(%error, "media WinRT initialization failed");
        return;
    }
    let mut paused = Vec::new();
    for pause in receiver {
        if pause {
            pause_playing(&mut paused);
        } else {
            resume_paused(&mut paused);
        }
    }
    // SAFETY: initialization succeeded on this thread.
    unsafe { RoUninitialize() };
}

fn pause_playing(paused: &mut Vec<Session>) {
    let result = (|| -> windows::core::Result<()> {
        let sessions = Manager::RequestAsync()?.join()?.GetSessions()?;
        for index in 0..sessions.Size()? {
            let session = match sessions.GetAt(index) {
                Ok(session) => session,
                Err(error) => {
                    tracing::warn!(%error, "media session unavailable");
                    continue;
                }
            };
            match pause_session(&session) {
                Ok(true) => paused.push(session),
                Ok(false) => {}
                Err(error) => tracing::warn!(%error, "media pause failed"),
            }
        }
        Ok(())
    })();
    if let Err(error) = result {
        tracing::warn!(%error, "media sessions unavailable");
    }
}

fn pause_session(session: &Session) -> windows::core::Result<bool> {
    if session.GetPlaybackInfo()?.PlaybackStatus()? != Status::Playing {
        return Ok(false);
    }
    if !session.TryPauseAsync()?.join()? {
        tracing::warn!("media session refused pause");
        return Ok(false);
    }
    Ok(true)
}

fn resume_paused(paused: &mut Vec<Session>) {
    for session in paused.drain(..) {
        let result = (|| -> windows::core::Result<()> {
            if session.GetPlaybackInfo()?.PlaybackStatus()? == Status::Paused
                && !session.TryPlayAsync()?.join()?
            {
                tracing::warn!("media session refused resume");
            }
            Ok(())
        })();
        if let Err(error) = result {
            tracing::warn!(%error, "media resume failed");
        }
    }
}
