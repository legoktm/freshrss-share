#![deny(clippy::all)]
use anyhow::Context;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use jiff::{Timestamp, fmt::strtime};
use mysql_async::prelude::*;
use mysql_async::{OptsBuilder, Pool, Row};
use rocket::http::Status;
use rocket::{State, fs::FileServer, get, launch, routes};
use rocket_dyn_templates::Template;
use serde::Serialize;
use std::{env, fs};

mod crypto;
mod csp;

struct SecretKey([u8; 32]);

#[derive(Debug, Serialize)]
struct Entry {
    title: String,
    author: Option<String>,
    content: String,
    link: String,
    date: String,
    feed_name: String,
    website: String,
}

impl TryFrom<Row> for Entry {
    type Error = anyhow::Error;

    fn try_from(mut row: Row) -> Result<Self, Self::Error> {
        let content: String = row.take("content").context("Missing content_bin field")?;

        let date_timestamp: i64 = row.take("date").context("Missing date field")?;
        let date = strtime::format(
            "%Y-%m-%d %H:%M:%S UTC",
            Timestamp::from_second(date_timestamp)?,
        )?;
        let author: String = row.take("author").context("Missing author field")?;
        let author = if let Some(author) = author.strip_prefix(';') {
            Some(author.to_string())
        } else if author.is_empty() {
            None
        } else {
            Some(author)
        };

        Ok(Entry {
            title: row.take("title").context("Missing title field")?,
            author,
            content,
            link: row.take("link").context("Missing link field")?,
            date,
            feed_name: row.take("feed_name").context("Missing feed_name field")?,
            website: row.take("website").context("Missing website field")?,
        })
    }
}

#[derive(Serialize)]
struct IndexTemplate {
    #[serde(flatten)]
    entry: Entry,
    expires_in_days: u64,
}

#[get("/?<p>")]
async fn index(
    p: String,
    pool: &State<Pool>,
    secret_key: &State<SecretKey>,
) -> Result<Template, Status> {
    // Decrypt the payload and get the ID
    let payload = crypto::decrypt_payload(&p, secret_key).map_err(|err| {
        eprintln!("Decryption error: {err}");
        Status::Unauthorized
    })?;
    let id = payload.id as i64;

    let mut conn = match pool.get_conn().await {
        Ok(conn) => conn,
        Err(err) => {
            eprintln!("Database connection error: {err}");
            return Err(Status::ServiceUnavailable);
        }
    };

    let result: Result<Option<Row>, _> = conn
        .exec_first(
            r#"
SELECT
    title,
    author,
    UNCOMPRESS(content_bin) AS content,
    link,
    date,
    kunal_feed.name as feed_name,
    kunal_feed.website as website
FROM kunal_entry
JOIN kunal_feed ON kunal_entry.id_feed = kunal_feed.id
WHERE kunal_entry.id = ?
"#,
            (id,),
        )
        .await;

    match result {
        Ok(Some(row)) => {
            let entry: Entry = match row.try_into() {
                Ok(entry) => entry,
                Err(err) => {
                    eprintln!("Row conversion error: {err}");
                    return Err(Status::InternalServerError);
                }
            };
            Ok(Template::render(
                "index",
                IndexTemplate {
                    entry,
                    expires_in_days: payload.expires_in_days,
                },
            ))
        }
        Ok(None) => Err(Status::NotFound),
        Err(err) => {
            eprintln!("Database query error: {err}");
            Err(Status::ServiceUnavailable)
        }
    }
}

#[derive(Serialize)]
struct SubmitTemplate {}

#[get("/submit")]
async fn submit() -> Template {
    Template::render("submit", SubmitTemplate {})
}

#[get("/")]
fn root() -> Status {
    Status::NotFound
}

#[launch]
async fn rocket() -> _ {
    // Get database URL from environment variable
    let hostname = env::var("MARIADB_HOST").unwrap_or("localhost".to_string());
    let user = env::var("MARIADB_USER").unwrap();
    let password = fs::read_to_string(env::var("MARIADB_PASSWORD_FILE").unwrap()).unwrap();
    let database = env::var("MARIADB_DATABASE").unwrap();

    // Get secret key from environment variable
    let secret_key_b64 =
        env::var("SECRET_KEY").expect("SECRET_KEY environment variable must be set");
    let secret_key_bytes = BASE64
        .decode(secret_key_b64.trim())
        .expect("SECRET_KEY must be valid base64");
    let secret_key = SecretKey(
        secret_key_bytes
            .try_into()
            .expect("SECRET_KEY must be exactly 32 bytes"),
    );

    // Create database connection pool
    let pool = Pool::new(
        OptsBuilder::default()
            .user(Some(user))
            .pass(Some(password))
            .db_name(Some(database))
            .ip_or_hostname(hostname)
            .tcp_port(3306),
    );

    // Test the connection
    match pool.get_conn().await {
        Ok(_) => println!("Successfully connected to database"),
        Err(e) => {
            eprintln!("Failed to connect to database: {e}");
        }
    }

    rocket::build()
        .manage(pool)
        .manage(secret_key)
        .mount("/", routes![index, root, submit])
        .mount("/static", FileServer::from("static"))
        .attach(Template::fairing())
        .attach(csp::CspFairing)
}
