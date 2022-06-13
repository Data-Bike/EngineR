use std::borrow::Borrow;
use std::error::Error;
use crate::controllers::secure::authentication::token::Token;
use crate::model::user::entity::user::User;

use sqlx::Error as Sqlx_Error;
use bcrypt::verify;


use std::fmt;
use std::io::Stderr;
use crate::controllers::secure::authentication::credentials::CheckCredentials;
use crate::model;
use crate::model::error::RepositoryError;

#[derive(Debug)]
pub struct AuthenticationError {
    source: AuthenticationErrorSideKick,
}

impl fmt::Display for AuthenticationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.source)
    }
}

impl Error for AuthenticationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}


impl From<Sqlx_Error> for AuthenticationError {
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

        Self { source: AuthenticationErrorSideKick }
    }
}

impl From<RepositoryError> for AuthenticationError {
    fn from(e: RepositoryError) -> Self {
        Self {
            source: AuthenticationErrorSideKick
        }
    }
}


#[derive(Debug)]
struct AuthenticationErrorSideKick;

impl fmt::Display for AuthenticationErrorSideKick {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Authentication error!")
    }
}

impl Error for AuthenticationErrorSideKick {}


pub struct Strategy {}

impl Strategy {
    pub fn new() -> Strategy {
        Self {}
    }

    pub async fn auth(token: &Token) -> Result<User, AuthenticationError> {
        let login = token.credentials.login.clone();
        match token.credentials.checkCredentials.borrow() {
            CheckCredentials::Password(password) => {
                let user = model::user::repository::repository::Repository::getUserByLogin(login).await?;
                let hash = user.password.clone();
                if verify(password, hash.as_str()).is_ok() {
                    return Ok(user);
                }
                Err(AuthenticationError {
                    source: AuthenticationErrorSideKick {}
                })
            }
            CheckCredentials::AccessToken(access_token) => {
                let user = model::user::repository::repository::Repository::getUserByLogin(login).await?;
                let hash = user.password.clone();
                if verify(access_token, hash.as_str()).is_ok() {
                    return Ok(user);
                }
                Err(AuthenticationError {
                    source: AuthenticationErrorSideKick {}
                })
            }
            CheckCredentials::OAuth(oauth) => {
                let user = model::user::repository::repository::Repository::getUserByLogin(login).await?;
                let hash = user.password.clone();
                if verify(oauth, hash.as_str()).is_ok() {
                    return Ok(user);
                }
                Err(AuthenticationError {
                    source: AuthenticationErrorSideKick {}
                })
            }
        }
    }
}