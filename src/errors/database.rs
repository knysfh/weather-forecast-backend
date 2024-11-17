use sqlx::Error as SqlxError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("Database connection error: {message} ")]
    ConnectionError { message: String, source: SqlxError },

    #[error("Database query error: {message} ")]
    QueryError { message: String, source: SqlxError },

    #[error("Pool timeout error: {0}")]
    TimeOut(#[source] SqlxError),

    #[error("Other database error: {0}")]
    Other(#[source] SqlxError),
}

impl From<SqlxError> for DbError {
    fn from(error: SqlxError) -> Self {
        match error {
            SqlxError::Database(db_err) => {
                if db_err.message().contains("connection") {
                    DbError::ConnectionError {
                        message: db_err.message().to_string(),
                        source: SqlxError::Database(db_err),
                    }
                } else {
                    DbError::QueryError {
                        message: db_err.message().to_string(),
                        source: SqlxError::Database(db_err),
                    }
                }
            }
            SqlxError::PoolTimedOut => DbError::TimeOut(error),
            _ => DbError::Other(error),
        }
    }
}
