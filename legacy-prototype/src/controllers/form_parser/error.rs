use std::error::Error;
use std::fmt::{Display, Formatter};
use sqlx::Error as Sqlx_Error;
use crate::model::error::RepositoryError;

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for ParseError {}

impl From<Sqlx_Error> for ParseError {
    fn from(e: Sqlx_Error) -> Self {
        let message = match e {
            Sqlx_Error::Configuration(e) => {
                e.to_string()
            }
            Sqlx_Error::Database(e) => { format!("Error returned from the database: '{}'", e.message()) }
            Sqlx_Error::Io(e) => { format!("Error communicating with the database backend: '{}'", e) }
            Sqlx_Error::Tls(e) => { format!("Error occurred while attempting to establish a TLS connection: '{}'", e) }
            Sqlx_Error::Protocol(e) => { format!("Unexpected or invalid data encountered while communicating with the database(Driver may be corrupted): '{}'", e) }
            Sqlx_Error::RowNotFound => { format!("No rows returned by a query that expected to return at least one row") }
            Sqlx_Error::TypeNotFound { type_name } => { format!("Type '{}' Not Found", type_name) }
            Sqlx_Error::ColumnIndexOutOfBounds { index, len } => { format!("Column index out of bounds: the len is {}, but the index is {}", len, index) }
            Sqlx_Error::ColumnNotFound(e) => { format!("No column found for the given name: '{}'", e) }
            Sqlx_Error::ColumnDecode { index, source } => { format!("Error occurred while decoding column {}: {}", index, source) }
            Sqlx_Error::Decode(e) => { format!("Error occurred while decoding a value: '{}'", e) }
            Sqlx_Error::PoolTimedOut => { format!("Pool Timed Out Error") }
            Sqlx_Error::PoolClosed => { format!("Pool Closed Error") }
            Sqlx_Error::WorkerCrashed => { format!("Worker Crashed Error") }
            Sqlx_Error::Migrate(e) => { format!("Migrate Error") }
            _ => { format!("Unknown SQLX DB ERROR") }
        };

        Self { message }
    }
}

impl From<RepositoryError> for ParseError {
    fn from(e: RepositoryError) -> Self {
        Self {
            message: e.message
        }
    }
}