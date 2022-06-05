use chrono::{DateTime, Utc};
use crate::controllers::secure::authentication::authentication::Authentication;
use crate::controllers::secure::authentication::credentials::Credentials;

pub enum IP {
    v4(String),
    v6(String),
}

pub struct Token {
    pub credentials: Credentials,
    pub date: DateTime<Utc>,
    pub ip: IP,
    authenticated: Option<bool>,
}

impl Token {
    pub fn new(credentials: Credentials, ip: IP) -> Token {
        Token {
            credentials,
            date: Utc::now(),
            ip,
            authenticated: None,
        }
    }

    pub fn authenticate(&mut self) {
        self.authenticated = Some(Authentication::auth(self))
    }
    pub fn is_authenticated(&self) -> Option<bool> {
        self.authenticated
    }
    pub fn is_allow(&self) -> bool {
        match self.authenticated {
            None => { false }
            Some(r) => { r }
        }
    }
}