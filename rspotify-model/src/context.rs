//! All objects related to context

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};

use std::collections::HashMap;
use std::time::Duration;

use crate::{
    millisecond_timestamp, option_duration_ms, CurrentlyPlayingType, Device, DisallowKey, IdBuf,
    PlayableIdBuf, PlayableItem, RepeatState, Type,
};

/// Context object
///
/// [Reference](https://developer.spotify.com/documentation/web-api/reference/#endpoint-get-recently-played)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Context<T: IdBuf> {
    pub id: T,
    pub href: String,
    pub external_urls: HashMap<String, String>,
    #[serde(rename = "type")]
    pub _type: Type,
}

/// Currently playing object
///
/// [Reference](https://developer.spotify.com/documentation/web-api/reference/#endpoint-get-recently-played)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CurrentlyPlayingContext<T: PlayableIdBuf> {
    pub context: Option<Context<T>>,
    #[serde(with = "millisecond_timestamp")]
    pub timestamp: DateTime<Utc>,
    #[serde(default)]
    #[serde(with = "option_duration_ms", rename = "progress_ms")]
    pub progress: Option<Duration>,
    pub is_playing: bool,
    pub item: Option<PlayableItem>,
    pub currently_playing_type: CurrentlyPlayingType,
    pub actions: Actions,
}
/// [Reference](https://developer.spotify.com/documentation/web-api/reference/#endpoint-get-information-about-the-users-current-playback)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CurrentPlaybackContext<T: PlayableIdBuf> {
    pub device: Device,
    pub repeat_state: RepeatState,
    pub shuffle_state: bool,
    pub context: Option<Context<T>>,
    #[serde(with = "millisecond_timestamp")]
    pub timestamp: DateTime<Utc>,
    #[serde(default)]
    #[serde(with = "option_duration_ms", rename = "progress_ms")]
    pub progress: Option<Duration>,
    pub is_playing: bool,
    pub item: Option<PlayableItem>,
    pub currently_playing_type: CurrentlyPlayingType,
    pub actions: Actions,
}

/// Actions object
///
/// [Reference](https://developer.spotify.com/documentation/web-api/reference/#endpoint-get-recently-played)
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Actions {
    pub disallows: Vec<DisallowKey>,
}

impl<'de> Deserialize<'de> for Actions {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct OriginalActions {
            pub disallows: HashMap<DisallowKey, bool>,
        }
        let orignal_actions = OriginalActions::deserialize(deserializer)?;
        Ok(Actions {
            disallows: orignal_actions
                .disallows
                .into_iter()
                .filter(|(_, value)| *value)
                .map(|(key, _)| key)
                .collect(),
        })
    }
}
