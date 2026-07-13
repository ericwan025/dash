//! The concrete voice service: a stateless transcript parser.

use crate::api::v1::VoiceApi;
use crate::error::ServiceError;
use crate::intent::Intent;
use async_trait::async_trait;
use dash_core::MediaAction;

/// The voice service. Stateless — it only interprets text.
#[derive(Debug, Default)]
pub struct VoiceService;

impl VoiceService {
    /// Create a new voice service.
    pub fn new() -> Self {
        VoiceService
    }
}

#[async_trait]
impl VoiceApi for VoiceService {
    async fn parse(&self, transcript: &str) -> Result<Intent, ServiceError> {
        parse_intent(transcript).ok_or_else(|| ServiceError::Unrecognized(transcript.to_string()))
    }
}

/// Pure transcript → [`Intent`] parser.
///
/// A deliberately small keyword/prefix grammar — enough to demonstrate the
/// pipeline without pulling in a real NLU engine. Public so it can be unit-tested
/// directly. Returns `None` for transcripts it does not recognize.
pub fn parse_intent(transcript: &str) -> Option<Intent> {
    let text = transcript.trim();
    let lower = text.to_lowercase();

    // --- Media transport (explicit keywords first) ---
    if lower.contains("pause") || lower.contains("stop") {
        return Some(Intent::Media(MediaAction::Pause));
    }
    if lower.contains("next") || lower.contains("skip") {
        return Some(Intent::Media(MediaAction::Next));
    }

    // --- Navigation ---
    let nav_verb = lower.contains("navigate")
        || lower.contains("drive")
        || lower.contains("take me")
        || lower.contains("go ");
    if nav_verb && lower.contains("home") {
        return Some(Intent::Navigate {
            destination: "Home".to_string(),
        });
    }
    for prefix in ["navigate to ", "drive to ", "take me to ", "go to ", "navigate "] {
        if let Some(idx) = lower.find(prefix) {
            let start = idx + prefix.len();
            if let Some(dest) = text.get(start..).map(str::trim) {
                if !dest.is_empty() {
                    return Some(Intent::Navigate {
                        destination: dest.to_string(),
                    });
                }
            }
        }
    }

    // --- Settings ---
    if let Some(rest) = lower.strip_prefix("set ") {
        if let Some((key, value)) = rest.split_once(" to ") {
            let (key, value) = (key.trim(), value.trim());
            if !key.is_empty() && !value.is_empty() {
                return Some(Intent::Setting {
                    key: key.to_string(),
                    value: value.to_string(),
                });
            }
        }
    }
    if lower.contains("mute") {
        return Some(Intent::Setting {
            key: "volume".to_string(),
            value: "0".to_string(),
        });
    }

    // --- Media play (broad match, kept last so "navigate to X" wins) ---
    if lower.contains("play") {
        return Some(Intent::Media(MediaAction::Play));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn parses_media_commands() {
        let v = VoiceService::new();
        assert_eq!(v.parse("play music").await.unwrap(), Intent::Media(MediaAction::Play));
        assert_eq!(v.parse("pause").await.unwrap(), Intent::Media(MediaAction::Pause));
        assert_eq!(v.parse("stop the music").await.unwrap(), Intent::Media(MediaAction::Pause));
        assert_eq!(v.parse("next track").await.unwrap(), Intent::Media(MediaAction::Next));
        assert_eq!(v.parse("skip").await.unwrap(), Intent::Media(MediaAction::Next));
    }

    #[test]
    fn parses_navigation() {
        assert_eq!(
            parse_intent("navigate to Pier 39"),
            Some(Intent::Navigate { destination: "Pier 39".into() })
        );
        assert_eq!(
            parse_intent("take me to the airport"),
            Some(Intent::Navigate { destination: "the airport".into() })
        );
        assert_eq!(
            parse_intent("go home"),
            Some(Intent::Navigate { destination: "Home".into() })
        );
    }

    #[test]
    fn parses_settings() {
        assert_eq!(
            parse_intent("set volume to 7"),
            Some(Intent::Setting { key: "volume".into(), value: "7".into() })
        );
        assert_eq!(
            parse_intent("set theme to dark"),
            Some(Intent::Setting { key: "theme".into(), value: "dark".into() })
        );
        assert_eq!(
            parse_intent("mute"),
            Some(Intent::Setting { key: "volume".into(), value: "0".into() })
        );
    }

    #[tokio::test]
    async fn unrecognized_transcript_errors() {
        let v = VoiceService::new();
        let err = v.parse("what is the meaning of life").await.unwrap_err();
        assert_eq!(err, ServiceError::Unrecognized("what is the meaning of life".into()));
    }

    #[test]
    fn navigate_beats_broad_play_match() {
        // "playhouse" contains "play" but this is clearly navigation.
        assert_eq!(
            parse_intent("navigate to the playhouse"),
            Some(Intent::Navigate { destination: "the playhouse".into() })
        );
    }
}
