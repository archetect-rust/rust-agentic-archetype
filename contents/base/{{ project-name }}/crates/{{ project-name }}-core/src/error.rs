use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("configuration error: {0}")]
    Config(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
{% if has_sqlite then %}    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
{% end %}
}
