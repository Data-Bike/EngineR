use std::sync::atomic::{AtomicUsize, Ordering};

use rocket::{routes, State};

use rocket::response::content::Html;
use rocket::get;
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
async fn get(id:usize) -> Html<String> {
    let object  = Repository::getObjectTypeFromObjectId(id.to_string()).await;
    Html(object.kind)
}
//
#[get("/reg/<login>")]
fn reg( login: &str) -> Html<String> {

    Html(format!("Token:  <br> Hashed:  <br> Login: "))
}
//
//
#[get("/all")]
fn all() -> Html<String> {

    // Html(all_users)
}
//
//
//
// #[database("my_pg_db")]
// struct PgDb(postgres::Client);
//
// #[launch]
// fn rocket() -> _ {
//
//     rocket::build().attach(PgDb::)
// }
//

pub fn stage() -> AdHoc {

    AdHoc::on_ignite("Managed user model", move |rocket| async move {
        rocket.mount("/object", routes![index,reg,all])

    })
}

#[cfg(test)] mod tests;