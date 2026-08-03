use chrono::{DateTime, Utc};
use serde::ser::*;
use serde::{Serialize, Deserialize};
use crate::model::user::entity::user::User;


// #[derive(Serialize, Deserialize, Debug,  Clone)]
pub struct Action {
    pub id: String,
    pub user: User,
    // pub project: Project,
    pub action: String,
    pub date: DateTime<Utc>,
}