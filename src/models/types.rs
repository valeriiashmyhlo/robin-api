use anyhow::Result;

//Do I need this?
pub type PostgresResult<T> = Result<T>;
pub type DatabaseResult<T> = Result<T, sqlx::Error>;
