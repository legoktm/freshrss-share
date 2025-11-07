use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Header;
use rocket::{Request, Response};

pub(crate) const INDEX_CSP: &str = "\
    default-src 'none'; \
    style-src 'self'; \
    script-src 'self'; \
    img-src 'self' https:; \
    frame-src 'self'; \
";

/// CSP policy for the submit route
/// - Allows styles from self (for style.css)
/// - Allows scripts from self (for submit.js and tweetnacl)
/// - Allows form actions to self
/// - No frames needed on this page
/// - Blocks objects and other potentially dangerous content
pub(crate) const SUBMIT_CSP: &str = "\
    default-src 'none'; \
    style-src 'self'; \
    script-src 'self'; \
";

pub(crate) struct CspFairing;

#[rocket::async_trait]
impl Fairing for CspFairing {
    fn info(&self) -> Info {
        Info {
            name: "CSP",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, request: &'r Request<'_>, response: &mut Response<'r>) {
        let path = request.uri().path();

        // Select appropriate CSP based on route
        let csp = if path.as_str() == "/submit" {
            SUBMIT_CSP
        } else {
            INDEX_CSP
        };

        // Set Content Security Policy
        response.set_header(Header::new("Content-Security-Policy", csp));
    }
}
