pub enum CheckCredentials {
    Password(String),
    AccessToken(String),
    OAuth(String),
}

pub struct Credentials {
    pub login: String,
    pub checkCredentials: CheckCredentials,
}