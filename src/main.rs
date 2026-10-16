#![deny(clippy::all)]
use anyhow::Context;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use jiff::{Timestamp, fmt::strtime};
use mysql_async::prelude::*;
use mysql_async::{OptsBuilder, Pool, Row};
use rocket::http::Status;
use rocket::shield::{Referrer, Shield};
use rocket::{State, fs::FileServer, get, launch, routes};
use rocket_dyn_templates::Template;
use serde::Serialize;
use std::{env, fs};

mod crypto;
mod csp;

struct SecretKey([u8; 32]);

/// FreshRSS prefixes its tables with the username
struct TablePrefix(String);

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

        // FreshRSS stores authors as a "; "-joined list with a leading ';'
        // (see Entry::authors()), e.g. ";Alice; Bob".
        let author: String = row.take("author").context("Missing author field")?;
        let authors: Vec<&str> = author
            .split(';')
            .map(str::trim)
            .filter(|author| !author.is_empty())
            .collect();
        let author = if authors.is_empty() {
            None
        } else {
            Some(authors.join(", "))
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
    table_prefix: &State<TablePrefix>,
) -> Result<Template, Status> {
    let prefix = &table_prefix.0;
    // Decrypt the payload and get the ID
    let payload = crypto::decrypt_payload(&p, secret_key).map_err(|err| {
        eprintln!("Decryption error: {err}");
        Status::Unauthorized
    })?;

    let mut conn = match pool.get_conn().await {
        Ok(conn) => conn,
        Err(err) => {
            eprintln!("Database connection error: {err}");
            return Err(Status::ServiceUnavailable);
        }
    };

    let query = format!(
        r#"
SELECT
    title,
    author,
    UNCOMPRESS(content_bin) AS content,
    link,
    date,
    {prefix}_feed.name as feed_name,
    {prefix}_feed.website as website
FROM {prefix}_entry
JOIN {prefix}_feed ON {prefix}_entry.id_feed = {prefix}_feed.id
WHERE {prefix}_entry.id = ?
"#
    );

    let result: Result<Option<Row>, _> = conn.exec_first(query, (payload.id,)).await;

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
    // Get database info
    let hostname = env::var("MARIADB_HOST").unwrap_or("localhost".to_string());
    let user = env::var("MARIADB_USER").expect("MARIADB_USER environment variable must be set");
    let password_file = env::var("MARIADB_PASSWORD_FILE")
        .expect("MARIADB_PASSWORD_FILE environment variable must be set");
    let password =
        fs::read_to_string(&password_file).expect("failed to read MARIADB_PASSWORD_FILE");
    let database =
        env::var("MARIADB_DATABASE").expect("MARIADB_DATABASE environment variable must be set");

    // FreshRSS table prefix (usually your FreshRSS username)
    let table_prefix = env::var("FRESHRSS_TABLE_PREFIX")
        .expect("FRESHRSS_TABLE_PREFIX environment variable must be set");
    if table_prefix.is_empty()
        || !table_prefix
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        panic!("FRESHRSS_TABLE_PREFIX must contain only ASCII letters, digits, and underscores");
    }

    // Get secret key
    let secret_key_file =
        env::var("SECRET_KEY_FILE").expect("SECRET_KEY_FILE environment variable must be set");
    let secret_key_b64 =
        fs::read_to_string(&secret_key_file).expect("failed to read SECRET_KEY_FILE");
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
            .pass(Some(password.trim()))
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
        .manage(TablePrefix(table_prefix))
        .mount("/", routes![index, root, submit])
        .mount("/static", FileServer::from("static"))
        .attach(Template::fairing())
        .attach(csp::CspFairing)
        .attach(Shield::default().enable(Referrer::NoReferrer))
}
