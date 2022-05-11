use chrono::{Date, DateTime, FixedOffset, TimeZone, Utc};
use crate::model::object::entity::object::Object;
use crate::model::user::entity::user::User;

pub struct Link {
    pub from:Object,
    pub to:Object,
    pub userLinked:User,
    pub userUnlinked:Option<User>,
    pub dateLinked:DateTime<FixedOffset>,
    pub dateUnlinked:Option<DateTime<FixedOffset>>
}