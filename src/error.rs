//! Error type for ftai-rs. See Task 2 for full implementation.

/// FTAI parse / serialize error. Expanded in Task 2.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Placeholder until Task 2 lands.
    #[error("ftai-rs error (placeholder)")]
    Placeholder,
}

/// Result alias.
pub type Result<T> = std::result::Result<T, Error>;
