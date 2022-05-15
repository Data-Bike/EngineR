use chrono::{Date, DateTime, FixedOffset, TimeZone, Utc};
use crate::model::object::entity::object::Object;
use crate::model::user::entity::user::User;

pub struct Link {
    pub object_from:Object,
    pub object_to:Object,
    pub user_created:User,
    pub user_deleted:Option<User>,
    pub date_created:DateTime<FixedOffset>,
    pub date_deleted:Option<DateTime<FixedOffset>>
}