use std::collections::LinkedList;
use std::vec;
use postgres::{Client, Error, NoTls, Row, RowIter};
use rocket::futures::future::err;
use rocket::shield::Feature::Accelerometer;
use crate::model::link::entity::link::Link;
use crate::model::object::entity::object::{Field, Object, ObjectType};
use crate::model::secure::entity::permission::{Access, Group, Permission, PermissionKind, PermissionLevel, PermissionsGroup};
use crate::model::user::entity::user::User;

pub struct Repository<'a> {
    db: &'a mut Client,
}

impl Repository<'_> {
    pub fn new(db: &'static mut Client) -> Self {
        Repository { db }
    }

    pub fn getPermissionsByGroup(&mut self, group: Group) -> Vec<Permission> {
        match self.db.query(format!("select * from permissions where group_alias={}", &group.alias).as_str(), &[]) {
            Ok(rows) => {
                let mut res: Vec<Permission> = Vec::new();
                for row in rows {
                    let access = match row.get::<_, &str>("access") {
                        "allow" => Access::allow,
                        "denied" => Access::deny,

                        _ => Access::deny
                    };
                    let alias = row.get::<_, String>("alias");
                    let level = match row.get::<_, &str>("access") {
                        "system" => PermissionLevel::system,
                        "object" => PermissionLevel::object,
                        "object_type" => PermissionLevel::object_type,
                        "link" => PermissionLevel::link,
                        "link_type" => PermissionLevel::link_type,

                        _ => PermissionLevel::system
                    };
                    let kind = match row.get::<_, &str>("access") {
                        "create" => PermissionKind::create,
                        "read" => PermissionKind::read,
                        "edit" => PermissionKind::edit,

                        _ => PermissionKind::read
                    };
                    let name = row.get::<_, String>("name");
                    let object = row.get::<_, String>("object");
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
            Err(error) => {
                panic!("{}", error.to_string().as_str())
            }
        }
    }

    pub fn getPermissionsGroupByPermissions(&mut self, permissions: Vec<Permission>) -> PermissionsGroup {
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

    pub fn getUserGroupsbyUser(&mut self, user: User) -> Vec<Group> {
        match self.db.query(format!("select * from user_group_permissions where user_alias={}", &user.login).as_str(), &[]) {
            Ok(rows) => {
                let mut res: Vec<Group> = Vec::new();
                for row in rows {
                    let alias = row.get::<_, String>("alias");
                    let name = row.get::<_, String>("name");
                    let permissions_vec = self.getPermissionsByGroup(Group {
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
                    });
                    let alias = row.get::<_, String>("alias");
                    let name = row.get::<_, String>("name");
                    let permissions = self.getPermissionsGroupByPermissions(permissions_vec);
                    res.push(Group {
                        alias,
                        name,
                        permissions,
                    });
                }
                res
            }
            Err(error) => {
                panic!("{}", error.to_string().as_str())
            }
        }
    }
}