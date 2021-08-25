//! This module defines the necessary elements in order to represent Spotify IDs
//! and URIs with type safety and no overhead.
//!
//! The trait [`Id`] is the central element of this module. It's implemented by
//! different kinds of ID ([`AlbumId`], [`EpisodeId`], etc), and implements the
//! logic to initialize and use IDs. [`Id`] is equivalent to a `&str` and
//! [`IdBuf`] to a `String`; you may use whichever suits you best.
//!
//! There are also group IDs, which may contain different kinds of IDs. For
//! example, `PlayableId` can hold IDs of either tracks or episodes. `AnyId`
//! can hold *any* kind of ID. These types are useful when an endpoint takes
//! different kinds of IDs as a parameter, or when the kind of ID you're dealing
//! with is known only at runtime.
//!
//! You can convert specific IDs into group ones with its `as_ref`
//! implementation, since it's a cheap type conversion.
//!
//! ## Examples
//!
//! If an endpoint requires a `TrackId`, you may pass it as:
//!
//! ```
//! fn pause_track(id: &TrackId) { /* ... */ }
//!
//! let id = TrackId::from_id("4iV5W9uYEdYUVa79Axb7Rh").unwrap();
//! pause_track(id);
//! ```
//!
//! Notice how it's type safe; the following example would fail at compile-time:
//!
//! ```compile_fail
//! fn pause_track(id: &TrackId) { /* ... */ }
//!
//! let id = EpisodeId::from_id("4iV5W9uYEdYUVa79Axb7Rh").unwrap();
//! pause_track(id);
//! ```
//!
//! And this would panic because it's a `TrackId` but its URI string specifies
//! it's an album (`spotify:album:xxxx`).
//!
//! ```should_panic
//! fn pause_track(id: &TrackId) { /* ... */ }
//!
//! let id = TrackId::from_uri("spotify:album:6akEvsycLGftJxYudPjmqK").unwrap();
//! pause_track(id);
//! ```
//!
//! A more complex example where an endpoint takes a vector of IDs of different
//! types:
//!
//! ```
//! fn track(id: &TrackId) { /* ... */ }
//! fn episode(id: &EpisodeId) { /* ... */ }
//! fn add_to_queue(id: &[&PlayableId]) { /* ... */ }
//!
//! let tracks = &[
//!     TrackId::from_uri("spotify:track:4iV5W9uYEdYUVa79Axb7Rh").unwrap(),
//!     TrackId::from_uri("spotify:track:5iKndSu1XI74U2OZePzP8L").unwrap(),
//! ];
//! let episodes = &[
//!     EpisodeId::from_id("0lbiy3LKzIY2fnyjioC11p").unwrap(),
//!     EpisodeId::from_id("4zugY5eJisugQj9rj8TYuh").unwrap(),
//! ];
//!
//! // First we get some info about the tracks and episodes
//! let track_info = tracks.into_iter().map(|id| track(id)).collect::<Vec<_>>()
//! let ep_info = tracks.into_iter().map(|id| episode(id)).collect::<Vec<_>>()
//! println!("Track info: {:?}", track_info);
//! println!("Episode info: {:?}", ep_info);
//!
//! // And then we play them
//! let mut playable = tracks;
//! playable.extend(episodes);
//! let playable = playable.into_iter().map(|id| id.as_ref()).collect::<Vec<_>>();
//! add_to_queue(playable);
//! ```

use serde::{Deserialize, Serialize};
use strum::Display;
use thiserror::Error;

use std::borrow::{Borrow, ToOwned};
use std::fmt::Debug;
use std::hash::Hash;
use std::ops::Deref;

use crate::Type;

/// Spotify id or URI parsing error
///
/// See also [`Id`](crate::idtypes::Id) for details.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Display, Error)]
pub enum IdError {
    /// Spotify URI prefix is not `spotify:` or `spotify/`
    InvalidPrefix,
    /// Spotify URI can't be split into type and id parts
    /// (e.g. it has invalid separator)
    InvalidFormat,
    /// Spotify URI has invalid type name, or id has invalid type in a given
    /// context (e.g. a method expects a track id, but artist id is provided)
    InvalidType,
    /// Spotify id is invalid (empty or contains non-alphanumeric characters)
    InvalidId,
}

pub trait Id {
    // TODO: add const for owned type and same in IdBuf?
    const TYPE: Type;

    /// Spotify object Id (guaranteed to be a string of alphanumeric characters)
    fn id(&self) -> &str;

    /// Initialize the Id without checking its validity.
    ///
    /// # Safety
    ///
    /// The string passed to this method must be made out of alphanumeric
    /// numbers only; otherwise undefined behaviour may occur.
    unsafe fn from_id_unchecked(id: &str) -> &Self;

    /// Spotify object URI in a well-known format: spotify:type:id
    ///
    /// Examples: `spotify:album:6IcGNaXFRf5Y1jc7QsE9O2`,
    /// `spotify:track:4y4VO05kYgUTo2bzbox1an`.
    fn uri(&self) -> String {
        format!("spotify:{}:{}", Self::TYPE, self.id())
    }

    /// Full Spotify object URL, can be opened in a browser
    ///
    /// Examples: `https://open.spotify.com/track/4y4VO05kYgUTo2bzbox1an`,
    /// `https://open.spotify.com/artist/2QI8e2Vwgg9KXOz2zjcrkI`.
    fn url(&self) -> String {
        format!("https://open.spotify.com/{}/{}", Self::TYPE, self.id())
    }

    /// Parse Spotify id or URI from string slice
    ///
    /// Spotify URI must be in one of the following formats:
    /// `spotify:{type}:{id}` or `spotify/{type}/{id}`.
    /// Where `{type}` is one of `artist`, `album`, `track`, `playlist`,
    /// `user`, `show`, or `episode`, and `{id}` is a non-empty
    /// alphanumeric string.
    /// The URI must be of given `T`ype, otherwise `IdError::InvalidType`
    /// error is returned.
    ///
    /// Examples: `spotify:album:6IcGNaXFRf5Y1jc7QsE9O2`,
    /// `spotify/track/4y4VO05kYgUTo2bzbox1an`.
    ///
    /// If input string is not a valid Spotify URI (it's not started with
    /// `spotify:` or `spotify/`), it must be a valid Spotify object id,
    /// i.e. a non-empty alphanumeric string.
    ///
    /// # Errors:
    ///
    /// - `IdError::InvalidType` - if `id_or_uri` is an URI, and it's type part
    ///    is not equal to `T`,
    /// - `IdError::InvalidId` - either if `id_or_uri` is an URI with invalid id
    ///    part, or it's an invalid id (id is invalid if it contains
    ///    non-alphanumeric characters),
    /// - `IdError::InvalidFormat` - if `id_or_uri` is an URI, and it can't be
    ///    split into type and id parts.
    fn from_id_or_uri(id_or_uri: &str) -> Result<&Self, IdError> {
        match Self::from_uri(id_or_uri) {
            Ok(id) => Ok(id),
            Err(IdError::InvalidPrefix) => Self::from_id(id_or_uri),
            Err(error) => Err(error),
        }
    }

    /// Parse Spotify id from string slice
    ///
    /// A valid Spotify object id must be a non-empty alphanumeric string.
    ///
    /// # Errors:
    ///
    /// - `IdError::InvalidId` - if `id` contains non-alphanumeric characters.
    fn from_id(id: &str) -> Result<&Self, IdError> {
        if id.chars().all(|ch| ch.is_ascii_alphanumeric()) {
            // Safe, we've just checked that the Id is valid.
            Ok(unsafe { Self::from_id_unchecked(id) })
        } else {
            Err(IdError::InvalidId)
        }
    }

    /// Parse Spotify URI from string slice
    ///
    /// Spotify URI must be in one of the following formats:
    /// `spotify:{type}:{id}` or `spotify/{type}/{id}`.
    /// Where `{type}` is one of `artist`, `album`, `track`, `playlist`, `user`,
    /// `show`, or `episode`, and `{id}` is a non-empty alphanumeric string.
    ///
    /// Examples: `spotify:album:6IcGNaXFRf5Y1jc7QsE9O2`,
    /// `spotify/track/4y4VO05kYgUTo2bzbox1an`.
    ///
    /// # Errors:
    ///
    /// - `IdError::InvalidPrefix` - if `uri` is not started with `spotify:`
    ///    or `spotify/`,
    /// - `IdError::InvalidType` - if type part of an `uri` is not a valid
    ///    Spotify type `T`,
    /// - `IdError::InvalidId` - if id part of an `uri` is not a valid id,
    /// - `IdError::InvalidFormat` - if it can't be splitted into type and
    ///    id parts.
    fn from_uri(uri: &str) -> Result<&Self, IdError> {
        let mut chars = uri
            .strip_prefix("spotify")
            .ok_or(IdError::InvalidPrefix)?
            .chars();
        let sep = match chars.next() {
            Some(ch) if ch == '/' || ch == ':' => ch,
            _ => return Err(IdError::InvalidPrefix),
        };
        let rest = chars.as_str();

        let (tpe, id) = rest
            .rfind(sep)
            .map(|mid| rest.split_at(mid))
            .ok_or(IdError::InvalidFormat)?;

        match tpe.parse::<Type>() {
            Ok(tpe) if tpe == Self::TYPE => Self::from_id(&id[1..]),
            _ => Err(IdError::InvalidType),
        }
    }
}

pub trait IdBuf: Id {}

/// This macro helps consistently define ID types. It contains a lot of code but
/// mostly it's just repetitive work that's not of much interest.
///
/// * The `$name` parameter indicates what type the ID is made out of (say,
///   `Artist`, `Album`...), which will then be used for its value in
///   `Id::_type`, which returns a `Type::$name`.
/// *
macro_rules! define_idtypes {
    ($($name:ident => $name_borrowed:ident, $name_owned:ident);+) => {
        $(
            #[doc = "Please refer to [`crate::idtypes`] for more information."]
            #[derive(Debug, PartialEq, Eq, Serialize, Hash)]
            pub struct $name_borrowed(str);

            impl Id for $name_borrowed {
                const TYPE: Type = Type::$name;

                unsafe fn from_id_unchecked(id: &str) -> &Self {
                    // Safe, because both types (str and this Id) share the same
                    // memory layout.
                    &*(id as *const str as *const Self)
                }

                fn id(&self) -> &str {
                    &self.0
                }
            }

            /// All types may be converted to `AnyId` without overhead
            impl AsRef<AnyId> for $name_borrowed {
                fn as_ref(&self) -> &AnyId {
                    // Safe, because the already intialized Id is assumed to be
                    // sound, so its ID is valid.
                    unsafe { AnyId::from_id_unchecked(&self.0) }
                }
            }

            /// Cheap conversion to `str`
            impl AsRef<str> for $name_borrowed {
                fn as_ref(&self) -> &str {
                    self.id()
                }
            }

            /// `Id`s may be borrowed as `str` the same way `Box<T>` may be
            /// borrowed as `T` or `String` as `str`
            impl Borrow<str> for $name_borrowed {
                fn borrow(&self) -> &str {
                    self.id()
                }
            }

            impl std::fmt::Display for $name_borrowed {
                fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                    // It only makes sense to show the full URI when the type is
                    // known. Otherwise the ID is enough.
                    match Self::TYPE {
                        Type::Unknown => write!(f, "{}", self.id()),
                        _ => write!(f, "{}", self.uri())
                    }
                }
            }

            impl ToOwned for $name_borrowed {
                type Owned = $name_owned;

                fn to_owned(&self) -> Self::Owned {
                    $name_owned((self.id()).to_owned())
                }
            }

            #[doc = "Please refer to [`crate::idtypes`] for more information."]
            #[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize, Hash)]
            pub struct $name_owned(String);

            /// A buffered ID is an ID after all, so it has to implement its
            /// trait as well, and also the `IdBuf` for additional functionality.
            impl Id for $name_owned {
                const TYPE: Type = Type::$name;

                unsafe fn from_id_unchecked(id: &str) -> &Self {
                    // Safe, because both types (str and this Id) share the same
                    // memory layout.
                    &*(id as *const str as *const Self)
                }

                fn id(&self) -> &str {
                    &self.0
                }
            }
            impl IdBuf for $name_owned {}

            /// Cheap conversion to its borrowed version
            impl AsRef<$name_borrowed> for $name_owned {
                fn as_ref(&self) -> &$name_borrowed {
                    // Safe, because the already intialized BufId is assumed to
                    // be sound, so its ID is valid.
                    unsafe { $name_borrowed::from_id_unchecked(&self.0) }
                }
            }

            /// Obviously, the owned ID (`IdBuf`) can be borrowed as the
            /// borrowed ID (`Id`)
            impl Borrow<$name_borrowed> for $name_owned {
                fn borrow(&self) -> &$name_borrowed {
                    self.as_ref()
                }
            }

            /// `Deref` helps make owned IDs more ergonomic
            impl Deref for $name_owned {
                type Target = $name_borrowed;

                fn deref(&self) -> &Self::Target {
                    self.as_ref()
                }
            }

            /// Owned IDs can also be used to convert from a `str`
            impl std::str::FromStr for $name_owned {
                type Err = IdError;

                fn from_str(s: &str) -> Result<Self, Self::Err> {
                    Self::from_id_or_uri(s).map(|id| id.to_owned())
                }
            }
        )+
    }
}

macro_rules! define_one_way_conversions {
    ($($from:ident => $into:ident),+) => {
        $(
            /// Cheap one way conversion from one type to a more generic one
            impl AsRef<$into> for $from {
                fn as_ref(&self) -> &$into {
                    // Safe, because the already intialized Id is assumed to be
                    // sound, so its ID is valid.
                    unsafe { $into::from_id_unchecked(self.id()) }
                }
            }
        )+
    }
}

define_idtypes!(
    // Basic types
    Artist => ArtistId, ArtistIdBuf;
    Album => AlbumId, AlbumIdBuf;
    Track => TrackId, TrackIdBuf;
    Playlist => PlaylistId, PlaylistIdBuf;
    User => UserId, UserIdBuf;
    Show => ShowId, ShowIdBuf;
    Episode => EpisodeId, EpisodeIdBuf;
    // Special Types: these cover a range of IDs instead of a single one,
    // covered in the `define_conversions!` block later on.
    Unknown => AnyId, AnyIdBuf;
    Unknown => PlayContextId, PlayContextIdBuf;
    Unknown => PlayableId, PlayableIdBuf
);

// Note that conversions to `AnyId` are already handled in `define_idtypes!`
define_one_way_conversions!(
    ArtistId => PlayContextId,
    AlbumId => PlayContextId,
    PlaylistId => PlayContextId,
    ShowId => PlayContextId,
    TrackId => PlayableId,
    EpisodeId => PlayableId
);

#[cfg(test)]
mod test {
    use super::*;

    // Valid values:
    const ID: &str = "4iV5W9uYEdYUVa79Axb7Rh";
    const URI: &str = "spotify:track:4iV5W9uYEdYUVa79Axb7Rh";
    const URI_SLASHES: &str = "spotify/track/4iV5W9uYEdYUVa79Axb7Rh";
    // Invalid values:
    const URI_EMPTY: &str = "spotify::4iV5W9uYEdYUVa79Axb7Rh";
    // Note that the API doesn't really have an 'Unknown' type.
    const URI_WRONGTYPE1: &str = "spotify:unknown:4iV5W9uYEdYUVa79Axb7Rh";
    const URI_WRONGTYPE2: &str = "spotify:something:4iV5W9uYEdYUVa79Axb7Rh";
    const URI_SHORT: &str = "track:4iV5W9uYEdYUVa79Axb7Rh";
    const URI_MIXED1: &str = "spotify/track:4iV5W9uYEdYUVa79Axb7Rh";
    const URI_MIXED2: &str = "spotify:track/4iV5W9uYEdYUVa79Axb7Rh";

    #[test]
    fn test_id_parse() {
        assert!(TrackId::from_id(ID).is_ok());
        assert_eq!(TrackId::from_id(URI), Err(IdError::InvalidId));
        assert_eq!(TrackId::from_id(URI_SLASHES), Err(IdError::InvalidId));
        assert_eq!(TrackId::from_id(URI_EMPTY), Err(IdError::InvalidId));
        assert_eq!(TrackId::from_id(URI_WRONGTYPE1), Err(IdError::InvalidId));
        assert_eq!(TrackId::from_id(URI_WRONGTYPE2), Err(IdError::InvalidId));
        assert_eq!(TrackId::from_id(URI_SHORT), Err(IdError::InvalidId));
        assert_eq!(TrackId::from_id(URI_MIXED1), Err(IdError::InvalidId));
        assert_eq!(TrackId::from_id(URI_MIXED2), Err(IdError::InvalidId));
    }

    #[test]
    fn test_uri_parse() {
        assert!(TrackId::from_uri(URI).is_ok());
        assert!(TrackId::from_uri(URI_SLASHES).is_ok());
        assert_eq!(TrackId::from_uri(ID), Err(IdError::InvalidPrefix));
        assert_eq!(TrackId::from_uri(URI_SHORT), Err(IdError::InvalidPrefix));
        assert_eq!(TrackId::from_uri(URI_EMPTY), Err(IdError::InvalidType));
        assert_eq!(TrackId::from_uri(URI_WRONGTYPE1), Err(IdError::InvalidType));
        assert_eq!(TrackId::from_uri(URI_WRONGTYPE2), Err(IdError::InvalidType));
        assert_eq!(TrackId::from_uri(URI_MIXED1), Err(IdError::InvalidFormat));
        assert_eq!(TrackId::from_uri(URI_MIXED2), Err(IdError::InvalidFormat));
    }

    #[test]
    fn test_id_or_uri_parse() {
        assert!(TrackId::from_id_or_uri(ID).is_ok());
        assert!(TrackId::from_id_or_uri(URI).is_ok());
        assert!(TrackId::from_id_or_uri(URI_SLASHES).is_ok());
        assert_eq!(TrackId::from_id_or_uri(URI_SHORT), Err(IdError::InvalidId));
        assert_eq!(
            TrackId::from_id_or_uri(URI_EMPTY),
            Err(IdError::InvalidType)
        );
        assert_eq!(
            TrackId::from_id_or_uri(URI_WRONGTYPE1),
            Err(IdError::InvalidType)
        );
        assert_eq!(
            TrackId::from_id_or_uri(URI_WRONGTYPE2),
            Err(IdError::InvalidType)
        );
        assert_eq!(
            TrackId::from_id_or_uri(URI_MIXED1),
            Err(IdError::InvalidFormat)
        );
        assert_eq!(
            TrackId::from_id_or_uri(URI_MIXED2),
            Err(IdError::InvalidFormat)
        );
    }

    #[test]
    fn test_unknown() {
        // Passing a Track ID to an Unknown ID type should work just fine.
        assert!(AnyId::from_id(ID).is_ok());
        assert!(AnyId::from_uri(URI).is_ok());
        assert!(AnyId::from_uri(URI_WRONGTYPE1).is_ok());
        assert!(AnyId::from_id_or_uri(ID).is_ok());
        assert!(AnyId::from_id_or_uri(URI).is_ok());

        // The given type must still be a variant of the `Type` enum, so it will
        // fail for invalid ones.
        assert_eq!(AnyId::from_uri(URI_EMPTY), Err(IdError::InvalidType));
        assert_eq!(AnyId::from_uri(URI_WRONGTYPE2), Err(IdError::InvalidType));

        // But it will still catch other kinds of error
        assert_eq!(AnyId::from_id_or_uri(URI_SHORT), Err(IdError::InvalidId));
        assert_eq!(
            AnyId::from_id_or_uri(URI_MIXED1),
            Err(IdError::InvalidFormat)
        );
        assert_eq!(
            AnyId::from_id_or_uri(URI_MIXED2),
            Err(IdError::InvalidFormat)
        );
    }
}
