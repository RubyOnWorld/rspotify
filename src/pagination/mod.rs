//! Utilities for automatic pagination requests. If the configured client is
//! asynchronous, it'll be  based on `futures::stream::Stream`, if it's
//! synchronous it'll just use `std::iter::Iterator`.

#[cfg(feature = "__sync")]
mod iter;
#[cfg(feature = "__async")]
mod stream;

#[cfg(feature = "__sync")]
pub use iter::{page_stream, StreamOrIterator};
#[cfg(feature = "__async")]
pub use stream::{page_stream, StreamOrIterator};