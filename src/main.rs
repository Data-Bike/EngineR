mod controllers;
mod model;

use crate::controllers::object;
use crate::controllers::login_logout;

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let rocket = rocket::build()
        .attach(object::stage())
        .attach(login_logout::stage());

    let rocket = rocket.ignite().await?;
    let res = rocket.launch().await?;
    Ok(())
}
