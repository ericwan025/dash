//! The concrete media service: state plus the [`v1::MediaApi`] implementation.

use crate::api::v1::MediaApi;
use crate::error::ServiceError;
use crate::types::{PlaybackState, Track};
use async_trait::async_trait;
use std::sync::Mutex;

/// The mutable inner state, guarded by a single lock.
#[derive(Debug)]
struct State {
    playlist: Vec<Track>,
    /// Index of the currently selected track. Only meaningful when the playlist
    /// is non-empty; otherwise ignored.
    index: usize,
    playing: bool,
}

/// The media playback service.
///
/// Holds a playlist and playback flags behind a [`Mutex`], exposing them through
/// the async [`v1::MediaApi`](crate::v1::MediaApi) trait. Because every method
/// takes `&self`, a single `MediaService` can be wrapped in an `Arc` and shared
/// across the runtime — the gateway calls it, and the bus loop (added next) calls
/// it too.
///
/// The lock is only ever held for the duration of a small, synchronous state
/// update — never across an `.await` — so a plain `std::sync::Mutex` is both
/// correct and cheaper than an async mutex here.
#[derive(Debug)]
pub struct MediaService {
    state: Mutex<State>,
}

impl MediaService {
    /// Create a service with the given playlist. Playback starts paused on the
    /// first track (or on nothing, if the playlist is empty).
    pub fn new(playlist: Vec<Track>) -> Self {
        MediaService {
            state: Mutex::new(State {
                playlist,
                index: 0,
                playing: false,
            }),
        }
    }

    /// Create a service pre-loaded with a few demo tracks, so the running system
    /// has something to play out of the box.
    pub fn with_demo_tracks() -> Self {
        MediaService::new(vec![
            Track::new("Highway Star", "Deep Purple"),
            Track::new("Radar Love", "Golden Earring"),
            Track::new("Life Is a Highway", "Tom Cochrane"),
        ])
    }

    /// Build a [`PlaybackState`] snapshot from the locked state.
    fn snapshot(state: &State) -> PlaybackState {
        PlaybackState {
            playing: state.playing,
            current: state.playlist.get(state.index).cloned(),
        }
    }
}

#[async_trait]
impl MediaApi for MediaService {
    async fn play(&self) -> Result<PlaybackState, ServiceError> {
        let mut state = self.state.lock().expect("media state lock poisoned");
        if state.playlist.is_empty() {
            return Err(ServiceError::EmptyPlaylist);
        }
        state.playing = true;
        Ok(Self::snapshot(&state))
    }

    async fn pause(&self) -> Result<PlaybackState, ServiceError> {
        let mut state = self.state.lock().expect("media state lock poisoned");
        // Pausing is always valid, even with an empty playlist.
        state.playing = false;
        Ok(Self::snapshot(&state))
    }

    async fn next_track(&self) -> Result<PlaybackState, ServiceError> {
        let mut state = self.state.lock().expect("media state lock poisoned");
        let len = state.playlist.len();
        if len == 0 {
            return Err(ServiceError::EmptyPlaylist);
        }
        state.index = (state.index + 1) % len;
        Ok(Self::snapshot(&state))
    }

    async fn select_track(&self, index: usize) -> Result<PlaybackState, ServiceError> {
        let mut state = self.state.lock().expect("media state lock poisoned");
        let len = state.playlist.len();
        if index >= len {
            return Err(ServiceError::TrackOutOfRange { index, len });
        }
        state.index = index;
        Ok(Self::snapshot(&state))
    }

    async fn now_playing(&self) -> Result<PlaybackState, ServiceError> {
        let state = self.state.lock().expect("media state lock poisoned");
        Ok(Self::snapshot(&state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo() -> MediaService {
        MediaService::with_demo_tracks()
    }

    #[tokio::test]
    async fn play_starts_playback_on_first_track() {
        let svc = demo();
        let state = svc.play().await.unwrap();
        assert!(state.playing);
        assert_eq!(state.current.unwrap().title, "Highway Star");
    }

    #[tokio::test]
    async fn pause_stops_playback() {
        let svc = demo();
        svc.play().await.unwrap();
        let state = svc.pause().await.unwrap();
        assert!(!state.playing);
    }

    #[tokio::test]
    async fn next_track_advances_and_wraps() {
        let svc = demo(); // 3 tracks
        assert_eq!(svc.next_track().await.unwrap().current.unwrap().title, "Radar Love");
        assert_eq!(
            svc.next_track().await.unwrap().current.unwrap().title,
            "Life Is a Highway"
        );
        // Wrap back to the first track.
        assert_eq!(
            svc.next_track().await.unwrap().current.unwrap().title,
            "Highway Star"
        );
    }

    #[tokio::test]
    async fn select_track_picks_the_requested_index() {
        let svc = demo();
        let state = svc.select_track(2).await.unwrap();
        assert_eq!(state.current.unwrap().title, "Life Is a Highway");
    }

    #[tokio::test]
    async fn select_track_out_of_range_errors() {
        let svc = demo();
        let err = svc.select_track(9).await.unwrap_err();
        assert_eq!(err, ServiceError::TrackOutOfRange { index: 9, len: 3 });
    }

    #[tokio::test]
    async fn play_on_empty_playlist_errors() {
        let svc = MediaService::new(vec![]);
        assert_eq!(svc.play().await.unwrap_err(), ServiceError::EmptyPlaylist);
    }

    #[tokio::test]
    async fn next_track_on_empty_playlist_errors() {
        let svc = MediaService::new(vec![]);
        assert_eq!(svc.next_track().await.unwrap_err(), ServiceError::EmptyPlaylist);
    }

    #[tokio::test]
    async fn pause_on_empty_playlist_is_ok() {
        let svc = MediaService::new(vec![]);
        let state = svc.pause().await.unwrap();
        assert!(!state.playing);
        assert!(state.current.is_none());
    }
}
