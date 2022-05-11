use crate::model::secure::entity::permission::{PermissionKind, PermissionLevel};

pub struct Token{
    pub requestLevel: PermissionLevel,
    pub requestKind: PermissionKind,
    pub system: String,
    pub object_type: Option<String>,
    pub object_type_field: Option<String>,
    pub object: Option<String>,
    pub link_type: Option<String>,
    pub link: Option<String>
}