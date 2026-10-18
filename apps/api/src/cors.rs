//! CORS fairing for local development / Docker demo (docs/16 §13).
//!
//! Allows all origins with credentials. In production, CORS is configured
//! via the reverse proxy (nginx/traefik), not the application.

use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::{Header, Method, Status};
use rocket::{Request, Response};

pub struct CorsFairing;

#[rocket::async_trait]
impl Fairing for CorsFairing {
    fn info(&self) -> Info {
        Info {
            name: "CORS",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, req: &'r Request<'_>, res: &mut Response<'r>) {
        let origin = req.headers().get_one("Origin").unwrap_or("*").to_string();

        res.set_header(Header::new("Access-Control-Allow-Origin", origin));
        res.set_header(Header::new(
            "Access-Control-Allow-Methods",
            "GET, POST, PUT, PATCH, DELETE, OPTIONS",
        ));
        res.set_header(Header::new(
            "Access-Control-Allow-Headers",
            "Content-Type, Authorization, X-Request-Id, X-Locale, X-Timezone, X-CSRF-Token, If-Match, Idempotency-Key, X-Entity-Type, X-Fields",
        ));
        res.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
        res.set_header(Header::new("Access-Control-Max-Age", "86400"));

        // Handle OPTIONS preflight — return 204 No Content
        if req.method() == Method::Options {
            res.set_status(Status::NoContent);
            res.set_header(Header::new("Content-Length", "0"));
        }
    }
}
