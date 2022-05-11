use chrono::{DateTime, Utc};
use crate::controllers::secure::authentication::credentials::Credentials;

pub struct Token{
   pub credentials:Credentials,
    pub date:DateTime<Utc>,
    pub ipv4:String,
    pub ipv6:String
}