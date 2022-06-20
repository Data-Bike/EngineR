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

#[get("/hello")]
async fn hello() -> RawJson<String> {
    return RawJson(format!("Hello!"));
}


pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Managing objects", move |rocket| async move {
        rocket.mount("/object", routes![get_object,add_object,hello])
    })
}

// #[cfg(test)]
// mod tests;

#[cfg(test)]
mod test {
    use rocket::local::blocking::Client;
    use rocket::http::Status;
    use rocket::uri;
    use crate::{ rocket_build};


    #[test]
    fn hello() {
        let client = Client::tracked(rocket_build()).expect("valid rocket instance");
        let mut response = client.get(uri!("/object/hello")).dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.into_string().unwrap(), "Hello!");
    }

    #[test]
    fn add_object() {
        let client = Client::tracked(rocket_build()).expect("valid rocket instance");
        let mut response = client.post(uri!("/object/add")).body("{\
            \"filled\":{\
                \"id\":\"1\",
                \"fields\":[
                    {
                        \"id\":\"1\",
                        \"alias\":\"lastname\",
                        \"kind\":\"varchar(255)\",
                        \"name\":\"lastname\",
                        \"value\":\"Platonov\",
                        \"require\":true,
                        \"index\":true,
                        \"preview\":true
                    },
                    {
                        \"id\":\"2\",
                        \"alias\":\"firstname\",
                        \"kind\":\"varchar(255)\",
                        \"name\":\"firstname\",
                        \"value\":\"Alexander\",
                        \"require\":true,
                        \"index\":true,
                        \"preview\":true
                    },
                    {
                        \"id\":\"3\",
                        \"alias\":\"patronymic\",
                        \"kind\":\"varchar(255)\",
                        \"name\":\"patronymic\",
                        \"value\":\"Alexanderovich\",
                        \"require\":true,
                        \"index\":true,
                        \"preview\":true
                    },
                    {
                        \"id\":\"4\",
                        \"alias\":\"birthday\",
                        \"kind\":\"varchar(255)\",
                        \"name\":\"birthday\",
                        \"value\":\"02-03-1988T00:00\",
                        \"require\":true,
                        \"index\":true,
                        \"preview\":true
                    },
                ],
                \"kind\":\"object\",
                \"alias\":\"fl\"
            },\
            \"date_created\":\"02-03-1988T02:30\",\
            \"user_created\":\"1\"
            \"hash\":\"\"\
        }").dispatch();

        assert_eq!(response.status(), Status::Ok);
        // assert_eq!(response.into_string().unwrap(), "Hello!");
    }
}