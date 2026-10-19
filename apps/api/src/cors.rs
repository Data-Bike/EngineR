//! CORS fairing (docs/16 §13).
//!
//! Origin allowlist — never reflect arbitrary `Origin` values, otherwise any
//! website could read credentialed API responses from a victim's browser.
//! Allowed origins come from `http.public_origin`, the built-in local dev
//! origins, and `ENGINER_CORS_ORIGINS` (comma-separated). In production, CORS
//! can additionally be restricted at the reverse proxy (nginx/traefik).

use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::{Header, Method, Status};
use rocket::{Request, Response};

/// Response fairing that emits CORS headers only for allowlisted origins.
pub struct CorsFairing {
    allowed: Vec<String>,
}

impl CorsFairing {
    /// Build the fairing from the configured public origin plus the
    /// `ENGINER_CORS_ORIGINS` env var (comma-separated) and local dev
    /// origins so the Nuxt dev server keeps working out of the box.
    pub fn new(public_origin: &str) -> Self {
        let mut allowed = vec![
            public_origin.trim_end_matches('/').to_string(),
            "http://localhost:3000".to_string(),
            "http://127.0.0.1:3000".to_string(),
            "http://localhost:8000".to_string(),
            "http://127.0.0.1:8000".to_string(),
        ];
        if let Ok(extra) = std::env::var("ENGINER_CORS_ORIGINS") {
            allowed.extend(
                extra
                    .split(',')
                    .map(|o| o.trim().trim_end_matches('/').to_string())
                    .filter(|o| !o.is_empty()),
            );
        }
        allowed.sort();
        allowed.dedup();
        Self { allowed }
    }
}

#[rocket::async_trait]
impl Fairing for CorsFairing {
    fn info(&self) -> Info {
        Info {
            name: "CORS",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, req: &'r Request<'_>, res: &mut Response<'r>) {
        let Some(origin) = req
            .headers()
            .get_one("Origin")
            .map(|o| o.trim().trim_end_matches('/'))
            .filter(|o| self.allowed.iter().any(|a| a == o))
        else {
            // Same-origin request or disallowed origin: no CORS headers.
            // Browsers then enforce the default cross-origin read block.
            if req.method() == Method::Options {
                res.set_status(Status::NoContent);
                res.set_header(Header::new("Content-Length", "0"));
            }
            return;
        };
        let origin = origin.to_string();

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
        res.set_header(Header::new("Vary", "Origin"));

        // Handle OPTIONS preflight — return 204 No Content
        if req.method() == Method::Options {
            res.set_status(Status::NoContent);
            res.set_header(Header::new("Content-Length", "0"));
        }
    }
}
