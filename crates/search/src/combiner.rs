use std::fmt;
use std::time::Duration;
use tracing::instrument;

use crate::SearchBackend;
use async_trait::async_trait;
use traits::{IndexSearcher, SearchResult};

#[derive(Debug)]
pub struct Searcher {
    pub(crate) started: std::time::Instant,
    pub backends: Vec<Box<dyn SearchBackend + Send>>,
}

#[derive(Debug)]
pub struct ErrorEntry {
    cause: crate::Error,
    backend: &'static str,
}

#[derive(Debug)]
pub struct Error {
    entries: Vec<ErrorEntry>,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for ErrorEntry { cause, backend } in &self.entries {
            f.write_fmt(format_args!("Backend: {backend} ran into error: {cause}"))?;
        }
        Ok(())
    }
}

impl std::error::Error for Error {}

#[async_trait]
impl IndexSearcher for Searcher {
    #[instrument(skip(self))]
    async fn search(
        &mut self,
        search_term: &str,
    ) -> (
        Vec<SearchResult>,
        Result<(), Box<dyn std::error::Error + Send>>,
    ) {
        use futures::stream::futures_unordered::FuturesUnordered;
        use futures::StreamExt;
        use itertools::Itertools;
        tracing::debug!("performing search for: {}", &search_term);

        if search_term.len() == 1 {
            self.started = std::time::Instant::now();
        }
        let search_started_recently = self.started.elapsed() < Duration::from_secs(6);
        if search_term.len() < 4 && search_started_recently {
            return (Vec::new(), Result::Ok(()));
        }

        let ignore_budget = false;
        let results: FuturesUnordered<_> = self
            .backends
            .iter_mut()
            .map(|b| b.search(&search_term, ignore_budget))
            .collect();
        let results: Vec<_> = results.collect().await;
        let (ok, err): (Vec<_>, Vec<_>) = results.into_iter().partition_result();
        let results: Vec<_> = ok
            .into_iter()
            .flatten()
            .dedup_by(|a, b| a.title == b.title)
            .collect();

        let err: Vec<_> = err
            .into_iter()
            .map(|cause| ErrorEntry {
                cause,
                backend: "todo",
            })
            .collect();

        let status = match err.is_empty() {
            true => Ok(()),
            false => {
                let err = Box::new(Error { entries: err });
                let err = err as Box<dyn std::error::Error + Send>;
                Err(err)
            }
        };

        (results, status)
    }
}
