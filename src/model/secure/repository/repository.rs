use std::collections::LinkedList;
use std::vec;
use rocket::futures::future::err;
use rocket::shield::Feature::Accelerometer;
use sqlx::Row;
use crate::controllers::pool::pool::sql;
use crate::model::link::entity::link::Link;
use crate::model::object::entity::object::{Field, Object, ObjectType};
use crate::model::secure::entity::permission::{Access, Group, Permission, PermissionKind, PermissionLevel, PermissionsGroup};
use crate::model::user::entity::user::User;

pub struct Repository {}

impl Repository {
    pub fn new() -> Self {
        Repository {}
    }

    pub async fn getPermissionsByGroup(group: Group) -> Vec<Permission> {
        let rows = sql(format!("select * from permissions where group_alias={}", &group.alias).as_str()).await;

        let mut res: Vec<Permission> = Vec::new();
        for row in rows {
            let access = match row.get::<&str, &str>("access") {
                "allow" => Access::allow,
                "denied" => Access::deny,

                _ => Access::deny
            };
            let alias = row.get::<String, &str>("alias");
            let level = match row.get::<&str, &str>("access") {
                "system" => PermissionLevel::system,
                "object" => PermissionLevel::object,
                "object_type" => PermissionLevel::object_type,
                "link" => PermissionLevel::link,
                "link_type" => PermissionLevel::link_type,

                _ => PermissionLevel::system
            };
            let kind = match row.get::<&str, &str>("access") {
                "create" => PermissionKind::create,
                "read" => PermissionKind::read,
                "edit" => PermissionKind::edit,

                _ => PermissionKind::read
            };
            let name = row.get::<String, &str>("name");
            let object = row.get::<String, &str>("object");
            res.push(Permission {
                access,
                alias,
                level,
                kind,
                name,
                object,
            });
        }
        res
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

    pub async fn getUserGroupsbyUser(user: User) -> Vec<Group> {
        let rows = sql(format!("select * from user_group_permissions where user_alias={}", &user.login).as_str()).await;

        let mut res: Vec<Group> = Vec::new();
        for row in rows {
            let alias = row.get::<String, &str>("alias");
            let name = row.get::<String, &str>("name");
            let permissions_vec = Self::getPermissionsByGroup(Group {
                alias,
                name,
                permissions: PermissionsGroup {
                    system: vec![],
                    object: vec![],
                    object_type: vec![],
                    object_type_field: vec![],
                    link: vec![],
                    link_type: vec![],
                },
            }).await;
            let alias = row.get::<String, &str>("alias");
            let name = row.get::<String, &str>("name");
            let permissions = Self::getPermissionsGroupByPermissions(permissions_vec);
            res.push(Group {
                alias,
                name,
                permissions,
            });
        }
        res
    }
}