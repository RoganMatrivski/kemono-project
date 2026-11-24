//! Offset paging shared by the post queries of every client.

use std::collections::VecDeque;
use std::future::Future;

use crate::error::Error;

/// Results per page; the offset `o` must be a multiple of this.
pub const PAGE_SIZE: u32 = 50;

/// A page with fewer than [`PAGE_SIZE`] items is the final one.
pub(crate) fn is_last_page(len: usize) -> bool {
    len < PAGE_SIZE as usize
}

/// Walks pages lazily. The caller supplies `fetch(offset)` on every call so the
/// cursor itself holds no client and no borrow.
#[derive(Debug)]
pub(crate) struct Cursor<T> {
    next_page: u32,
    max_pages: Option<u32>,
    buf: VecDeque<T>,
    done: bool,
}

impl<T> Cursor<T> {
    pub fn new(max_pages: Option<u32>) -> Self {
        Self {
            next_page: 0,
            max_pages,
            buf: VecDeque::new(),
            done: false,
        }
    }

    /// The next item, fetching another page when the buffer is empty.
    /// After an error the failed page is retried by the next call.
    pub async fn next<F, Fut>(&mut self, mut fetch: F) -> Result<Option<T>, Error>
    where
        F: FnMut(u32) -> Fut,
        Fut: Future<Output = Result<Vec<T>, Error>>,
    {
        loop {
            if let Some(item) = self.buf.pop_front() {
                return Ok(Some(item));
            }
            if self.done || self.max_pages.is_some_and(|m| self.next_page >= m) {
                self.done = true;
                return Ok(None);
            }
            let batch = fetch(self.next_page * PAGE_SIZE).await?;
            self.next_page += 1;
            self.done = is_last_page(batch.len());
            self.buf.extend(batch);
        }
    }

    /// Drains everything that is left.
    pub async fn collect<F, Fut>(&mut self, mut fetch: F) -> Result<Vec<T>, Error>
    where
        F: FnMut(u32) -> Fut,
        Fut: Future<Output = Result<Vec<T>, Error>>,
    {
        let mut all = Vec::new();
        while let Some(item) = self.next(&mut fetch).await? {
            all.push(item);
        }
        Ok(all)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pages(
        sizes: &'static [usize],
    ) -> impl FnMut(u32) -> std::future::Ready<Result<Vec<u32>, Error>> {
        move |offset| {
            assert_eq!(offset % PAGE_SIZE, 0);
            let n = sizes
                .get((offset / PAGE_SIZE) as usize)
                .copied()
                .unwrap_or(0);
            std::future::ready(Ok((0..n as u32).map(|i| offset + i).collect()))
        }
    }

    #[test]
    fn last_page_rule() {
        assert!(is_last_page(0));
        assert!(is_last_page(49));
        assert!(!is_last_page(50));
    }

    #[tokio::test]
    async fn stops_on_short_page() {
        let all = Cursor::new(None)
            .collect(pages(&[50, 50, 7]))
            .await
            .unwrap();
        assert_eq!(all.len(), 107);
        assert_eq!(all[50], 50); // in order, second page starts at offset 50
    }

    #[tokio::test]
    async fn full_page_then_empty() {
        let all = Cursor::new(None).collect(pages(&[50])).await.unwrap();
        assert_eq!(all.len(), 50);
    }

    #[tokio::test]
    async fn max_pages_caps_requests() {
        let all = Cursor::new(Some(2))
            .collect(pages(&[50, 50, 50]))
            .await
            .unwrap();
        assert_eq!(all.len(), 100);
        assert!(
            Cursor::<u32>::new(Some(0))
                .collect(pages(&[50]))
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn next_yields_lazily_and_retries_after_error() {
        let mut c = Cursor::new(None);
        let mut calls = 0;
        let mut fetch = |offset: u32| {
            calls += 1;
            let r = if calls == 1 {
                Err(Error::NotFound)
            } else {
                Ok((0..3).map(|i| offset + i).collect())
            };
            std::future::ready(r)
        };
        assert!(c.next(&mut fetch).await.is_err());
        assert_eq!(c.next(&mut fetch).await.unwrap(), Some(0));
        assert_eq!(c.next(&mut fetch).await.unwrap(), Some(1));
    }
}
