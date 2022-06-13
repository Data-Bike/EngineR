use rocket::{routes};
use rocket::response::content::{RawHtml, RawJson};
use rocket::get;
use rocket::post;
use rocket::fairing::AdHoc;
use serde_json::value::to_value;
use crate::model::object::entity::object::Object;
use crate::model::object::repository::repository::Repository;


#[get("/get/<id>")]
async fn get_object(id: usize) -> RawHtml<String> {
    let object = Repository::hydrateFilledObjectType(id.to_string()).await.ok();
    match object {
        None => { RawHtml(format!("ERROR")) }
        Some(o) => {
            RawHtml(
                match to_value(o) {
                    Ok(x) => { x }
                    Err(e) => { return RawHtml("ERROR".to_string()); }
                }.to_string()
            )
        }
    }
}

#[post("/add", data = "<object>")]
async fn add_object(object: Object) -> RawJson<String> {
    let id = Repository::createObject(&object).await.ok();
    match id {
        None => { RawJson(format!("ERROR")) }
        Some(i) => { RawJson(i) }
    }
}

#[post("/search", data = "<object>")]
async fn search_object(object: Object) -> RawJson<String> {
    let res = Repository::searchObject(&object).await.ok();
    match res {
        None => { RawJson(format!("ERROR")) }
        Some(r) => {
            RawJson(
                match to_value(r) {
                    Ok(x) => { x }
                    Err(e) => { return RawJson("ERROR".to_string()); }
                }.to_string())
        }
    }
}


pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Managing objects", move |rocket| async move {
        rocket.mount("/object", routes![get_object,add_object])
    })
}

#[cfg(test)]
mod tests;