
use crate::model::secure::entity::permission::Group;

// #[derive(Serialize, Deserialize, Debug,  Clone)]
pub struct User {
    pub id: String,
    pub login: String,
    pub token_hashed: String,
    pub groups:Vec<Group>
}