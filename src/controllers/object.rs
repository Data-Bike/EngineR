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


// pub use diesel;
// pub use postgres;
// pub use r2d2_postgres;
// pub use r2d2;
//
// #[get("/token")]
// fn index(model: &State<&PgDb>) -> Html<String> {
//     Html(format!("Token:  <br> Hashed: "))
// }
//
// #[get("/reg/<login>")]
// fn reg(model: &State<&PgDb>, login: &str) -> Html<String> {
//
//     Html(format!("Token:  <br> Hashed:  <br> Login: "))
// }
//
//
// #[get("/all")]
// fn all(model: &State<&PgDb>) -> Html<String> {
//     let users_iter = model.database.tokens.iter();
//     let mut all_users = "".to_string();
//     users_iter.for_each(|item| {
//         // hashed=std::str::from_utf8(item.unwrap().0.as_ref()).unwrap();
//         let unwrap_item = item.unwrap();
//         let h = unwrap_item.0.as_ref();
//         let u = unwrap_item.1;
//         all_users = format!("{}<br>{:?}",
//                             all_users,
//                             User::from_bin(u).to_json().to_string()
//         );
//     });
//     Html(all_users)
// }
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
// pub fn stage(model: &'static PgDb) -> AdHoc {
//
//     AdHoc::on_ignite("Managed user model", move |rocket| async move {
//         rocket.mount("/object", routes![index,reg,all])
//             .manage(model)
//     })
// }

#[cfg(test)] mod tests;