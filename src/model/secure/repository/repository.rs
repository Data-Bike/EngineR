use std::collections::LinkedList;
use std::vec;
use rocket::futures::future::err;
use rocket::shield::Feature::Accelerometer;
use sqlx::Row;
use sqlx::Error as Sqlx_Error;
use crate::controllers::pool::pool::{sql, sql_one};
use crate::model::error::RepositoryError;
use crate::model::link::entity::link::Link;
use crate::model::object::entity::object::{Field, Object, ObjectType};
use crate::model::secure::entity::permission::{Access, Group, Permission, PermissionKind, PermissionLevel, PermissionsGroup};
use crate::model::user::entity::user::User;

pub struct Repository {}

impl Repository {
    pub fn new() -> Self {
        Repository {}
    }

    pub async fn getPermissionsByGroup(group: Group) -> Result<Vec<Permission>, RepositoryError> {
        Ok(Self::getPermissionsById(match group.id {
            None => { return Err(RepositoryError { message: format!("Group must has id") }); }
            Some(g) => { g }
        }.as_str()).await?)
    }

    pub async fn getPermissionsById(id: &str) -> Result<Vec<Permission>, RepositoryError> {
        let rows = sql(format!("select * from permissions where group={}", id).as_str()).await?;

        let mut res: Vec<Permission> = Vec::new();
        for row in rows {
            let access = match row.get::<&str, &str>("access") {
                "allow" => Access::allow,
                "denied" => Access::deny,

                _ => Access::deny
            };
            let alias = row.get::<String, &str>("alias");
            let level = match row.get::<&str, &str>("level") {
                "system" => PermissionLevel::system,
                "object" => PermissionLevel::object,
                "object_type" => PermissionLevel::object_type,
                "link" => PermissionLevel::link,
                "link_type" => PermissionLevel::link_type,

                _ => PermissionLevel::system
            };
            let kind = match row.get::<&str, &str>("kind") {
                "create" => PermissionKind::create,
                "read" => PermissionKind::read,
                "edit" => PermissionKind::edit,

                _ => PermissionKind::read
            };
            let name = row.get::<String, &str>("name");
            let object = row.get::<String, &str>("object");
            let id = Some(row.get::<String, &str>("id"));
            res.push(Permission {
                id,
                access,
                alias,
                level,
                kind,
                name,
                object,
            });
        }
        Ok(res)
    }


    pub fn getPermissionsGroupByPermissions(permissions: Vec<Permission>) -> PermissionsGroup {
        let mut system = Vec::<Permission>::new();
        let mut object = Vec::<Permission>::new();
        let mut object_type = Vec::<Permission>::new();
        let mut object_type_field = Vec::<Permission>::new();
        let mut link = Vec::<Permission>::new();
        let mut link_type = Vec::<Permission>::new();

        for permission in permissions {
            match permission.level {
                PermissionLevel::system => {
                    system.push(permission)
                }
                PermissionLevel::object => {
                    object.push(permission)
                }
                PermissionLevel::object_type => {
                    object_type.push(permission)
                }
                PermissionLevel::object_type_field => {
                    object_type_field.push(permission)
                }
                PermissionLevel::link => {
                    link.push(permission)
                }
                PermissionLevel::link_type => {
                    link_type.push(permission)
                }
            }
        };

        PermissionsGroup {
            system,
            object,
            object_type,
            object_type_field,
            link,
            link_type,
        }
    }

    pub async fn getUserGroupsbyUser(user: User) -> Result<Vec<Group>, RepositoryError> {
        let rows = sql(format!("select g.* from user_group join group on user_group.user_id={} and user_group.group_id=group.id ", match user.id.as_ref() {
            None => { return Err(RepositoryError { message: format!("User must has id") }); }
            Some(i) => { i }
        }).as_str()).await?;

        let mut res: Vec<Group> = Vec::new();
        for row in rows {
            let alias = row.get::<String, &str>("alias");
            let name = row.get::<String, &str>("name");
            let level = row.get::<String, &str>("level");
            let id = Some(row.get::<String, &str>("id"));
            let permissions_vec = Self::getPermissionsByGroup(Group {
                id: id.clone(),
                alias,
                level: level.clone(),
                name,
                permissions: PermissionsGroup {
                    system: vec![],
                    object: vec![],
                    object_type: vec![],
                    object_type_field: vec![],
                    link: vec![],
                    link_type: vec![],
                },
            }).await?;
            let alias = row.get::<String, &str>("alias");
            let name = row.get::<String, &str>("name");
            let permissions = Self::getPermissionsGroupByPermissions(permissions_vec);
            res.push(Group {
                id,
                alias,
                level,
                name,
                permissions,
            });
        }
        Ok(res)
    }
    pub async fn getGroupById(id: &str) -> Result<Group, RepositoryError> {
        let group_row = sql_one(format!("select * from group where id = {} limit 1", id).as_str()).await?;
        Ok(Group {
            alias: group_row.get::<String, &str>("alias"),
            name: group_row.get::<String, &str>("name"),
            level: group_row.get::<String, &str>("level"),
            id: Some(group_row.get::<String, &str>("id")),
            permissions: Self::getPermissionsGroupByPermissions(
                Self::getPermissionsById(id).await?
            ),
        })
    }
}