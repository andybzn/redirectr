# redirectr

A lightweight, fast, self-hosted shortlink redirection service written in Rust using [Axum](https://github.com/tokio-rs/axum) and [Sqlx](https://github.com/launchbadge/sqlx) with SQLite.

---

## Features

- **Fast HTTP 302 Redirection:** Instant lookups backed by SQLite.
- **Secure Link Management:** Create, update, and delete links, all protected by Bearer token authentication with constant-time equality checks to mitigate timing attacks.
- **URL Scheme Validation:** Only accepts `http://` and `https://` URLs to prevent protocol abuse.
- **Default Fallback:** Visiting `/` redirects to a configurable landing page or repository URL.
- **Health Check Endpoint:** Probes the SQLite database schema for zero-downtime health monitors and container orchestrators.
- **Automatic Migrations:** Embedded SQLx migrations executed at application startup.
- **Structured Logging:** Powered by `tracing` with configurable log levels.

---

## Quick Start (Docker)

The fastest way to run `redirectr` is via Docker. The image already binds `SERVER_IP=0.0.0.0` and points `DATABASE_URL` at `/data`; mount a volume there to ensure links persist across container restarts.

### Docker CLI

```bash
docker run -d \
  --name redirectr \
  -p 3333:3333 \
  -v redirectr_data:/data \
  -e ADMIN_TOKEN="your-super-secret-token" \
  -e DEFAULT_URL="https://example.com" \
  docker.io/andybzn/redirectr:latest
```

### Docker Compose

Create a `docker-compose.yml` file:

```yaml
services:
    redirectr:
        image: docker.io/andybzn/redirectr:latest
        container_name: redirectr
        restart: unless-stopped
        ports:
            - "3333:3333"
        environment:
            ADMIN_TOKEN: "your-super-secret-token"
            DEFAULT_URL: "https://example.com"
            RUST_LOG: "redirectr=info,sqlx=warn"
        volumes:
            - redirectr_data:/data

volumes:
    redirectr_data:
```

Run with:

```bash
docker compose up -d
```

---

## Configuration

`redirectr` is configured entirely through environment variables:

| Variable       | Required | Default                                | Description                                                    |
| -------------- | -------- | -------------------------------------- | -------------------------------------------------------------- |
| `ADMIN_TOKEN`  | **Yes**  | —                                      | Bearer token required to create, update, or delete links via `POST /`, `PATCH /`, and `DELETE /{code}`. |
| `SERVER_IP`    | No       | `127.0.0.1`                            | IP address to bind the HTTP server to (`0.0.0.0` for Docker).  |
| `SERVER_PORT`  | No       | `3333`                                 | TCP port the HTTP server listens on.                           |
| `DATABASE_URL` | No       | `sqlite://redirectr.sqlite`            | SQLite connection URL (file created automatically if missing). |
| `MAX_CONN`     | No       | `9`                                    | Maximum SQLite connection pool size.                           |
| `DEFAULT_URL`  | No       | `https://github.com/andybzn/redirectr` | Fallback destination for `GET /`.                              |
| `RUST_LOG`     | No       | `redirectr=debug,sqlx=warn`            | `tracing` subscriber log filter level.                         |

---

## API Reference

### 1. Resolve Shortlink

- **Route:** `GET /{code}`
- **Response:**
    - `302 Found` with `Location: <target_url>` on success
    - `404 Not Found` if the code does not exist

```bash
curl -i http://localhost:3333/my-link
```

---

### 2. Create Shortlink

- **Route:** `POST /?code=<code>&url=<target_url>`
- **Headers:** `Authorization: Bearer <ADMIN_TOKEN>`
- **Response:**
    - `201 Created` on success
    - `400 Bad Request` if the target URL is invalid or uses an unsupported scheme
    - `401 Unauthorized` if the bearer token is missing or invalid
    - `409 Conflict` if the `code` is already in use

```bash
curl -i -X POST "http://localhost:3333/?code=gh&url=https%3A%2F%2Fgithub.com%2Fandybzn%2Fredirectr" \
  -H "Authorization: Bearer your-super-secret-token"
```

---

### 3. Update Shortlink

- **Route:** `PATCH /?code=<code>&url=<target_url>`
- **Headers:** `Authorization: Bearer <ADMIN_TOKEN>`
- **Response:**
    - `204 No Content` on success
    - `400 Bad Request` if the target URL is invalid or uses an unsupported scheme
    - `401 Unauthorized` if the bearer token is missing or invalid
    - `404 Not Found` if the `code` does not exist

```bash
curl -i -X PATCH "http://localhost:3333/?code=gh&url=https%3A%2F%2Fgithub.com%2Fandybzn%2Fredirectr2" \
  -H "Authorization: Bearer your-super-secret-token"
```

---

### 4. Delete Shortlink

- **Route:** `DELETE /{code}`
- **Headers:** `Authorization: Bearer <ADMIN_TOKEN>`
- **Response:**
    - `204 No Content` on success, whether or not the code existed
    - `401 Unauthorized` if the bearer token is missing or invalid

```bash
curl -i -X DELETE "http://localhost:3333/gh" \
  -H "Authorization: Bearer your-super-secret-token"
```

---

### 5. Root Fallback

- **Route:** `GET /`
- **Response:** `302 Found` redirecting to the configured `DEFAULT_URL`.

```bash
curl -i http://localhost:3333/
```

---

### 6. Health Check

- **Route:** `GET /health`
- **Response:**
    - `200 OK` when the database and `links` table are accessible
    - `503 Service Unavailable` on database failure

```bash
curl -i http://localhost:3333/health
```

---

## Development

### Prerequisites

- [Rust](https://rustup.rs/) (Rust 2024 edition / stable toolchain)
- SQLite
- _(Optional)_ [devenv](https://devenv.sh/) / [Nix](https://nixos.org/)

### Building & Running Locally

```bash
# Clone the repository
git clone https://github.com/andybzn/redirectr.git
cd redirectr

# Run locally with cargo
ADMIN_TOKEN="dev-secret-token" cargo run
```

### Using Devenv

If you use `devenv`, all dependencies and tasks are pre-configured:

```bash
# Enter development shell
devenv shell

# Run all test and lint tasks
devenv tasks run app:tests
```

### Testing & Linting

```bash
# Run unit and integration tests
cargo test

# Run Clippy with strict checks
cargo clippy --all-targets -- -D warnings

# Check code formatting
cargo fmt --check

# Re-generate SQLx query metadata cache (if SQL queries are modified)
cargo sqlx prepare
```

---

## License

This project is licensed under the terms of the [MIT License](./LICENSE)
