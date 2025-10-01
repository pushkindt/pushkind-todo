# pushkind-template

`pushkind-template` is a template repository for building Pushkind web services powered by Rust, Actix Web, Diesel, and Tera on top of Bootstrap 5.3 for the frontend.

## Features

- Actix Web server with identity and session management
- SQLite database access via Diesel ORM
- REST API endpoints for template management
- Tera templates for server-rendered pages

## Running locally

1. Install [Rust](https://www.rust-lang.org/tools/install).
2. Set the required environment variables:
   - `DATABASE_URL` (e.g. `app.db`)
   - `SECRET_KEY` for session encryption
   - `AUTH_SERVICE_URL` a url of the authentication service
   - Optional: `PORT`, `ADDRESS`, `DOMAIN`
3. Run database migrations with `diesel migration run` (requires `diesel-cli`).
4. Start the server:

```bash
cargo run
```

The service listens on `http://127.0.0.1:8080` by default.

## Testing

Run the test suite with:

```bash
cargo test
```
