mod controllers;
mod model;

use rocket::{Build, Rocket};
use crate::controllers::{link, object, object_type, user};
use crate::controllers::login_logout;

pub fn rocket_build() -> Rocket<Build> {
    rocket::build()
        .attach(object::stage())
        .attach(login_logout::stage())
        .attach(object_type::stage())
        .attach(link::stage())
        .attach(user::stage())
}

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let _ = rocket_build()
        .ignite().await?
        .launch().await?;
    Ok(())
}
