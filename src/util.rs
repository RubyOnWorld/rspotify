use chrono::prelude::*;
use getrandom::getrandom;

/// Convert datetime to unix timestamp
pub(in crate) fn datetime_to_timestamp(elapsed: u32) -> i64 {
    let utc: DateTime<Utc> = Utc::now();
    utc.timestamp() + i64::from(elapsed)
}

/// Generate `length` random chars
pub(in crate) fn generate_random_string(length: usize) -> String {
    let alphanum: &[u8] =
        "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789".as_bytes();
    let mut buf = vec![0u8; length];
    getrandom(&mut buf).unwrap();
    let range = alphanum.len();

    buf.iter()
        .map(|byte| alphanum[*byte as usize % range] as char)
        .collect()
}

#[cfg(feature = "__async")]
pub mod stream {
    use crate::model::Page;
    use std::error::Error;

    /// Alias for either std::iter::Iterator<Item = T> or futures::stream::Stream<Item = T>
    /// depending on whether sync or async mode is enabled
    pub trait StreamOrIterator<T>: futures::stream::Stream<Item = T> {}
    impl<T, I: futures::stream::Stream<Item = T>> StreamOrIterator<T> for I {}

    pub(in crate) fn page_stream<'a, T, E, Fut, Function>(
        f: Function,
        page_size: u32,
    ) -> impl futures::stream::Stream<Item = Result<T, E>> + 'a
    where
        T: Unpin + 'static,
        E: Error + Unpin + 'static,
        Fut: futures::future::Future<Output = Result<Page<T>, E>>,
        Function: 'a + Fn(u32, u32) -> Fut,
    {
        use async_stream::stream;
        let mut offset = 0;
        stream! {
            loop {
                let page = f(page_size, offset).await?;
                offset += page.items.len() as u32;
                for item in page.items {
                    yield Ok(item);
                }
                if page.next.is_none() {
                    break;
                }
            }
        }
    }
}

#[cfg(feature = "__sync")]
pub mod stream {
    use crate::model::Page;
    use std::error::Error;

    /// Alias for either std::iter::Iterator<Item = T> or futures::stream::Stream<Item = T>
    /// depending on whether sync or async mode is enabled
    pub trait StreamOrIterator<T>: std::iter::Iterator<Item = T> {}
    impl<T, I: std::iter::Iterator<Item = T>> StreamOrIterator<T> for I {}

    pub(in crate) fn page_stream<'a, T, E, Function>(
        f: Function,
        page_size: u32,
    ) -> impl Iterator<Item = Result<T, E>> + 'a
    where
        T: Unpin + 'static,
        E: Error + Unpin + 'static,
        Function: 'a + Fn(u32, u32) -> Result<Page<T>, E>,
    {
        let pages = PageIterator {
            f,
            offset: 0,
            done: false,
            page_size,
        };
        pages.flat_map(|result| ResultIter::new(result.map(|page| page.items.into_iter())))
    }

    /// Iterator that repeatedly calls a function that returns a page until an empty page is returned
    struct PageIterator<T, E, Function>
    where
        T: Unpin + 'static,
        E: Error + Unpin + 'static,
        Function: Fn(u32, u32) -> Result<Page<T>, E>,
    {
        f: Function,
        offset: u32,
        done: bool,
        page_size: u32,
    }

    impl<T, E, Function> Iterator for PageIterator<T, E, Function>
    where
        T: Unpin + 'static,
        E: Error + Unpin + 'static,
        Function: Fn(u32, u32) -> Result<Page<T>, E>,
    {
        type Item = Result<Page<T>, E>;

        fn next(&mut self) -> Option<Self::Item> {
            if self.done {
                None
            } else {
                match (self.f)(self.page_size, self.offset) {
                    Ok(page) if page.items.is_empty() => {
                        self.done = true;
                        None
                    }
                    Ok(page) => {
                        self.offset += page.items.len() as u32;
                        Some(Ok(page))
                    }
                    Err(e) => Some(Err(e)),
                }
            }
        }
    }

    /// Helper to transform a Result<Iterator<Item = T>, E> into an Iterator<Item = Result<T, E>>
    struct ResultIter<T, E, I: Iterator<Item = T>> {
        inner: Option<I>,
        err: Option<E>,
    }

    impl<T, E, I: Iterator<Item = T>> ResultIter<T, E, I> {
        pub fn new(res: Result<I, E>) -> Self {
            match res {
                Ok(inner) => ResultIter {
                    inner: Some(inner),
                    err: None,
                },
                Err(err) => ResultIter {
                    inner: None,
                    err: Some(err),
                },
            }
        }
    }

    impl<T, E, I: Iterator<Item = T>> Iterator for ResultIter<T, E, I> {
        type Item = Result<T, E>;

        fn next(&mut self) -> Option<Self::Item> {
            match (self.err.take(), &mut self.inner) {
                (Some(err), _) => Some(Err(err)),
                (None, Some(inner)) => inner.next().map(Ok),
                _ => None, // Error already taken
            }
        }
    }
}
