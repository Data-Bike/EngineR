use std::sync::atomic::{AtomicUsize, Ordering};

use rocket::{routes, State};

use rocket::response::content::RawHtml;
use rocket::get;
use rocket::post;
use rocket::launch;
use rocket::fairing::AdHoc;
// use serde_hjson::value::ToJson;
// use crate::init::model::{Database, ModelApp, Secure, User};
use crate::model::user::entity::user::User;
// use rocket_sync_db_pools::database;
// use rocket_sync_db_pools::postgres;
use crate::model;
use crate::model::object::repository::repository::Repository;


#[get("/get/<id>")]
async fn get(id: usize) -> RawHtml<String> {
    let object = Repository::getObjectTypeFromObjectId(id.to_string()).await;
    RawHtml(object.kind)
}

#[post("/add")]
async fn add() -> RawHtml<String> {
    // Repository::createObject()
    // let object  = Repository::getObjectTypeFromObjectId(id.to_string()).await;
    RawHtml("object.kind".to_string())
}
//

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Managing objects", move |rocket| async move {
        rocket.mount("/object", routes![get,add])
    })
}

#[cfg(test)]
mod tests;