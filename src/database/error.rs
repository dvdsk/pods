#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Internal error in underlying database: {0:?}")]
    Internal(#[from] sled::Error),
    #[error("Episode is not in database")]
    NotInDatabase,
    #[error("Podcast was already added")]
    PodcastAlreadyAdded,
}
