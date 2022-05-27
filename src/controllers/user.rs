use std::sync::atomic::{AtomicUsize, Ordering};

use rocket::{routes, State};
// use rocket::response::content::Html;
use rocket::fairing::AdHoc;
// use serde_hjson::value::ToJson;
// use crate::init::model::{Database, ModelApp, Secure, User};

//
// #[get("/token")]
// fn index(model: &State<&ModelApp>) -> Html<String> {
//     let token = Secure::token();
//     Html(format!("Token: {} <br> Hashed: {}", token, Secure::hashed(token.as_str())))
// }
//
// #[get("/reg/<login>")]
// fn reg(model: &State<&ModelApp>, login: &str) -> Html<String> {
//     let token = Secure::token();
//     let hashed = Secure::hashed(token.as_str());
//     let hashed_to_view = hashed.clone();
//     model.user.reg(login.to_string(), hashed);
//     Html(format!("Token: {} <br> Hashed: {} <br> Login: {}", token, hashed_to_view, login))
// }
//
//
// #[get("/all")]
// fn all(model: &State<&ModelApp>) -> Html<String> {
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

// pub fn stage(model: &'static ModelApp) -> AdHoc {
//     AdHoc::on_ignite("Managed user model", move |rocket| async move {
//         rocket.mount("/user", routes![index,reg,all])
//             .manage(model)
//     })
// }

#[cfg(test)] mod tests;