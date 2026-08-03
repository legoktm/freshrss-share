# FreshRSS Share - RSS Entry Viewer

A Rust/Rocket web server that queries and displays entries from a FreshRSS MariaDB database.

## Features

- Query FreshRSS entries by ID
- Display article content with metadata (title, author, date)
- Clean, responsive HTML interface
- Direct links to original articles

## Prerequisites

- Rust (1.70 or later)
- MariaDB/MySQL with FreshRSS database
- Access to the `kunal_entry` table

## Installation

1. Clone or navigate to the project directory:
```bash
cd freshrss-share
```

2. Create a `.env` file with your database credentials:
```bash
cp .env.example .env
```

3. Edit `.env` and update the `DATABASE_URL`:
```env
DATABASE_URL=mysql://username:password@localhost:3306/freshrss
```

Replace:
- `username` with your MariaDB username
- `password` with your MariaDB password
- `localhost` with your database host
- `3306` with your database port (if different)
- `freshrss` with your database name

## Build and Run

### Development Mode

```bash
cargo run
```

### Production Build

```bash
cargo build --release
./target/release/freshrss-share
```

## Usage

1. Start the server:
```bash
cargo run
```

2. Open your browser and navigate to:
```
http://localhost:8000
```

3. Enter an entry ID and click "View Entry" or navigate directly to:
```
http://localhost:8000/entry?id=123
```

Replace `123` with any valid entry ID from your `kunal_entry` table.

## Configuration

### Database Connection

The `DATABASE_URL` supports various formats:

- TCP connection: `mysql://user:pass@host:port/database`
- Unix socket: `mysql://user:pass@localhost/database?socket=/path/to/socket`

## Database Schema

The server expects a `kunal_entry` table with the following structure:

```sql
CREATE TABLE kunal_entry (
    id bigint(20) NOT NULL PRIMARY KEY,
    guid varchar(767) NOT NULL,
    title varchar(8192) NOT NULL,
    author varchar(1024),
    content_bin mediumblob,
    link varchar(16383) NOT NULL,
    date bigint(20),
    lastSeen bigint(20),
    hash binary(16),
    is_read tinyint(1) NOT NULL DEFAULT 0,
    is_favorite tinyint(1) NOT NULL DEFAULT 0,
    id_feed int(11),
    tags varchar(2048),
    attributes text
);
```

**Note:** The application currently uses only: `id`, `title`, `author`, `content_bin`, `link`, and `date`.

## API Endpoints

- `GET /` - Home page with search form
- `GET /entry?id=<id>` - Display entry with the specified ID

## Troubleshooting

### Database Connection Issues

If you see "Failed to connect to database":
1. Verify your `DATABASE_URL` in `.env`
2. Check that MariaDB is running: `sudo systemctl status mariadb`
3. Test connection: `mysql -u username -p -h localhost freshrss`

### Permission Denied

Ensure your database user has SELECT permissions:
```sql
GRANT SELECT ON freshrss.kunal_entry TO 'username'@'localhost';
FLUSH PRIVILEGES;
```

### Port Already in Use

If port 8000 is busy, change it in `.env`:
```env
ROCKET_PORT=8080
```

## Development

### Dependencies

- `rocket` - Web framework
- `rocket_dyn_templates` - Tera template engine integration
- `mysql_async` - Async MySQL/MariaDB driver
- `tokio` - Async runtime
- `serde` - Serialization framework
- `jiff` - Modern date and time library
- `dotenv` - Environment variable management

### Project Structure

```
freshrss-share/
├── Cargo.toml          # Rust dependencies
├── .env                # Environment configuration (not in git)
├── .env.example        # Example configuration
├── README.md           # This file
├── src/
│   └── main.rs         # Main application code
├── templates/          # Tera HTML templates
│   ├── base.html.tera
│   ├── index.html.tera
│   ├── entry.html.tera
│   └── not_found.html.tera
└── static/             # Static assets (CSS, JS, images)
    └── style.css
```

## License

This project is provided as-is for personal use with FreshRSS installations.

## Contributing

Feel free to submit issues or pull requests for improvements.
