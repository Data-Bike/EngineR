use std::error::Error;
use crate::controllers::secure::authentication::token::Token;
use crate::model::user::entity::user::User;
use crate::model::user::repository::repository::Repository;
use bcrypt::verify;


use std::fmt;
use std::io::Stderr;
use crate::model;

#[derive(Debug)]
pub struct AuthenticationError {
    source: AuthenticationErrorSideKick,
}

impl fmt::Display for AuthenticationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}",self.source)
    }
}

impl Error for AuthenticationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
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



pub struct Strategy{
    pub user_model:Repository
}

impl Strategy {

    pub fn new(user_model:Repository)->Strategy{
        Self{
            user_model
        }
    }

    pub async fn auth(&mut self, token:&Token)->Result<User,AuthenticationError>{
        let login = token.credentials.login.clone();
        let password = token.credentials.password.clone();
        let user = model::user::repository::repository::Repository::getUserByLogin(login).await;
        let hash = user.password.clone();
        if verify(password,hash.as_str()).is_ok(){
            return Ok(user)
        }
        Err(AuthenticationError{
           source: AuthenticationErrorSideKick {}
        })
    }
}