use std::collections::LinkedList;
use std::vec;
use async_std::task::{JoinHandle, spawn};
use futures::executor::block_on;
use rocket::futures::future::err;
use rocket::shield::Feature::Accelerometer;
use sqlx::Row;
use sqlx::Error as Sqlx_Error;
use crate::{cache_it, remove_it_from_cache};
use crate::model::app::pool::pool::{delete, insert, sql, sql_one, update};
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
        let rows = sql(format!("select * from \"permission\" where \"group_id\"='{}'", id).as_str()).await?;

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
                "object_type_field" => PermissionLevel::object_type_field,
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
            let id = Some(row.get::<i64, &str>("id").to_string());
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
        let rows = sql(format!("select g.* from \"r_user_group\" ug join \"group\" g on ug.user_id={} and ug.group_id=g.id ", match user.id.as_ref() {
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

    pub async fn getUserGroupsbyUserId(id: String) -> Result<Vec<Group>, RepositoryError> {
        let rows = sql(format!("select g.* from \"r_user_group\" ug join \"group\" g on ug.user_id={} and ug.group_id=g.id ", id.as_str()).as_str()).await?;

        let mut res: Vec<Group> = Vec::new();
        for row in rows {
            let alias = row.get::<String, &str>("alias");
            let name = row.get::<String, &str>("name");
            let level = row.get::<String, &str>("level");
            let id = Some(row.get::<i64, &str>("id").to_string());
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
        let ids = id.to_string();
        cache_it!(&ids,group_by_id,{
            let group_row = sql_one(format!("select * from \"group\" where \"id\" = '{}' limit 1", id).as_str()).await?;
            Group {
                alias: group_row.get::<String, &str>("alias"),
                name: group_row.get::<String, &str>("name"),
                level: group_row.get::<String, &str>("level"),
                id: Some(group_row.get::<i64, &str>("id").to_string()),
                permissions: Self::getPermissionsGroupByPermissions(
                    Self::getPermissionsById(id).await?
                ),
            }
        })
    }

    pub fn groupToNameValues(group: &Group) -> Vec<(String, String)> {
        vec![
            ("name".to_string(), group.name.clone()),
            ("alias".to_string(), group.alias.clone()),
            ("level".to_string(), group.level.clone()),
        ]
    }

    pub fn permissionsGroupToNameValues(permissions: &PermissionsGroup, id: &String) -> Vec<Vec<(String, String)>> {
        let mut nv: Vec<Vec<(String, String)>> = vec![];
        for permission in permissions.link.iter() {
            nv.push(
                vec![
                    ("name".to_string(), permission.name.to_string()),
                    ("level".to_string(), permission.level.to_string()),
                    ("alias".to_string(), permission.alias.to_string()),
                    ("object".to_string(), permission.object.to_string()),
                    ("kind".to_string(), permission.kind.to_string()),
                    ("access".to_string(), permission.access.to_string()),
                    ("group_id".to_string(), id.to_string()),
                ]
            );
        }

        for permission in permissions.object.iter() {
            nv.push(
                vec![
                    ("name".to_string(), permission.name.to_string()),
                    ("level".to_string(), permission.level.to_string()),
                    ("alias".to_string(), permission.alias.to_string()),
                    ("object".to_string(), permission.object.to_string()),
                    ("kind".to_string(), permission.kind.to_string()),
                    ("access".to_string(), permission.access.to_string()),
                    ("group_id".to_string(), id.to_string()),
                ]
            );
        }

        for permission in permissions.object_type.iter() {
            nv.push(
                vec![
                    ("name".to_string(), permission.name.to_string()),
                    ("level".to_string(), permission.level.to_string()),
                    ("alias".to_string(), permission.alias.to_string()),
                    ("object".to_string(), permission.object.to_string()),
                    ("kind".to_string(), permission.kind.to_string()),
                    ("access".to_string(), permission.access.to_string()),
                    ("group_id".to_string(), id.to_string()),
                ]
            );
        }

        for permission in permissions.object_type_field.iter() {
            nv.push(
                vec![
                    ("name".to_string(), permission.name.to_string()),
                    ("level".to_string(), permission.level.to_string()),
                    ("alias".to_string(), permission.alias.to_string()),
                    ("object".to_string(), permission.object.to_string()),
                    ("kind".to_string(), permission.kind.to_string()),
                    ("access".to_string(), permission.access.to_string()),
                    ("group_id".to_string(), id.to_string()),
                ]
            );
        }

        for permission in permissions.link_type.iter() {
            nv.push(
                vec![
                    ("name".to_string(), permission.name.to_string()),
                    ("level".to_string(), permission.level.to_string()),
                    ("alias".to_string(), permission.alias.to_string()),
                    ("object".to_string(), permission.object.to_string()),
                    ("kind".to_string(), permission.kind.to_string()),
                    ("access".to_string(), permission.access.to_string()),
                    ("group_id".to_string(), id.to_string()),
                ]
            );
        }

        for permission in permissions.system.iter() {
            nv.push(
                vec![
                    ("name".to_string(), permission.name.to_string()),
                    ("level".to_string(), permission.level.to_string()),
                    ("alias".to_string(), permission.alias.to_string()),
                    ("object".to_string(), permission.object.to_string()),
                    ("kind".to_string(), permission.kind.to_string()),
                    ("access".to_string(), permission.access.to_string()),
                    ("group_id".to_string(), id.to_string()),
                ]
            );
        }
        nv
    }

    pub async fn createGroup(group: &Group) -> Result<String, RepositoryError> {
        let nv_group = Self::groupToNameValues(group);
        let id = insert("group".to_string(), nv_group).await?;
        let nv_permissions = Self::permissionsGroupToNameValues(&group.permissions, &id);
        let mut futures: Vec<JoinHandle<_>> = vec![];

        for nv_permission in nv_permissions {
            futures.push(spawn(insert("permission".to_string(), nv_permission)));
        }

        for future in futures {
            block_on(future)?;
        }
        Ok(id)
    }

    pub async fn updateGroup(group: &Group) -> Result<(), RepositoryError> {
        let id = match group.id.as_ref() {
            None => { return Err(RepositoryError { message: format!("Group must has id") }); }
            Some(id) => { id }
        };
        remove_it_from_cache!(id,group_by_id);
        let mut futures: Vec<JoinHandle<_>> = vec![];
        let exist_group = Self::getGroupById(id.as_str()).await?;
        futures.push(
            spawn(
                delete(
                    "permission".to_string(),
                    vec![],
                    vec![("group_id".to_string(), "=".to_string(), id.to_string())],
                )
            )
        );
        if exist_group.alias != group.alias
            || exist_group.level != group.level
            || exist_group.name != group.name
        {
            let nv_group = Self::groupToNameValues(group);
            futures.push(
                spawn(
                    update("group".to_string(),
                           nv_group,
                           vec![("id".to_string(), "=".to_string(), id.to_string())])
                )
            );
        };
        let nv_permissions = Self::permissionsGroupToNameValues(&group.permissions, id);
        for nv_permission in nv_permissions {
            futures.push(
                spawn(
                    insert("permission".to_string(), nv_permission)
                )
            );
        }

        for future in futures {
            block_on(future)?;
        }

        Ok(())
    }
}