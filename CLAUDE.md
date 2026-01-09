# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Development Commands

**Prerequisites:**
- Rust nightly-2025-08-01 (requires nightly features: `cfg_select`, `unsafe_pinned`)
- PostgreSQL database
- Tailwind CSS CLI (`tailwindcss` in PATH)
- Git binaries (`git-upload-pack`, `git-receive-pack`)

**Build:**
```bash
cargo build --release
```

**Run:**
```bash
cargo run
```

**Configuration:**
- Copy `config.example.toml` to `config.toml` and adjust settings
- Environment variable `DATABASE_URL` overrides config file database settings
- `.env` file is automatically loaded via dotenvy

**Docker:**
```bash
docker build -t conduit .
docker run -p 8080:8080 -p 8022:8022 conduit
```

## Architecture Overview

### Dual-Server Model
Conduit runs two concurrent servers in a single process:
- **HTTP Server** (port 8080): Web UI using Axum framework
- **SSH Server** (port 8022): Git operations using custom libssh wrapper

Both servers share the same `AppState` containing database pool, config, and cancellation tokens for graceful shutdown.

### Concurrency & Shutdown
- Single-threaded Tokio runtime (`current_thread`) with 8 blocking threads
- `CancellationToken` (ct) propagated to all async tasks for graceful shutdown
- `TaskTracker` (tt) ensures all tasks complete before process exit
- 10-second grace period after cancellation before forced shutdown

### SSH Git Flow (src/ssh.rs)
1. Accept SSH connection via custom libssh wrapper
2. Handle key exchange and authentication (currently keys loaded as empty vec - TODO)
3. Parse Git command: `git-upload-pack` or `git-receive-pack`
4. Spawn child process with Git binary from PATH
5. Bridge SSH channel ↔ child process stdio:
   - Channel data → child stdin
   - Child stdout → channel (non-stderr)
   - Child stderr → channel (stderr flag)
6. Handle buffering when channel not writable (32-byte buffers)
7. Send EOF and exit status when child completes
8. Close channel on completion

**Important:** Data copying happens in the select loop after `session.wait()` returns, not inside channel event handlers.

### Database Transaction Retry Logic (src/db.rs)
- Custom transaction wrapper with exponential backoff
- Automatically retries on PostgreSQL serialization conflicts:
  - `40001`: serialization_failure
  - `40P01`: deadlock_detected
  - `23505`: unique_violation
  - `23P01`: exclusion_violation
- Backoff schedule: 10ms, 25ms, 50ms, 100ms, 100ms (max 5 retries)
- Connection pool: 2-8 connections, 5s acquire timeout, 300s idle timeout

### State Management (src/state.rs)
`AppState` is an `Arc<AppStateInner>` containing:
- `db: PgPool` - PostgreSQL connection pool
- `config: Config` - Loaded from config.toml
- `cancel_token: CancellationToken` - For graceful shutdown
- `task_tracker: TaskTracker` - Tracks running tasks

Implements `FromRequestParts` for Axum extraction and `Deref` for ergonomic field access.

### Route Organization (src/routes/)
Routes are modular and merged in `routes()`:
- `assets::routes()` - Static file serving
- `autoreload::routes()` - Dev mode auto-reload (debug builds only)
- `hub::routes()` - User profiles at `/~{username}`
- `meta::routes()` - User metadata (profile, SSH keys)
- `paste::routes()` - Code paste creation/viewing
- `shell::routes()` - HTML document wrapper with Maud

### Data Models (src/model/)
- `user` - User accounts and authentication
- `session` - Session tokens with expiration
- `paste` - Code snippets with visibility levels

Each model encapsulates database queries using SQLx compile-time checked queries.

### Build Process (build.rs)
- Generates CSS via Tailwind CLI (`tailwindcss -i styles/index.css`)
- Computes version from Git: `dev-{8-char-hash}[-dirty]`
- CSS minification enabled in release builds (`-m` flag)
- Reruns on changes to: `build.rs`, `.git/HEAD`, `src/`, `styles/`

### Authentication Middleware (src/middleware/auth.rs)
- Extracts session from HTTP-only cookie
- Validates session against database
- Injects `Option<Session>` into request extensions
- Session contains `token` and `user_id`

## Key Technical Details

**Database Schema:**
- `users`: id (identity), username (unique), password_hash
- `user_keys`: SSH public keys (ssh-ed25519), references users
- `sessions`: token (PK), user_id, expires timestamp
- `pastes`: id (text PK), user_id, visibility (public/unlisted/private)
- `paste_files`: paste_id + filename (composite PK), content

**Unique ID Generation:**
- Pastes use `unique_string` utility with collision handling
- Configurable collision behavior (likely in paste model)

**SSH Command Parsing:**
Regex: `^([a-zA-Z\-]+) '/?([a-zA-Z0-9]+)/([\.\-a-zA-Z0-9]+\.git)'$`
- Captures: command, username, repo name
- Only allows: `git-upload-pack`, `git-receive-pack`
- Repository path: `{config.git.repository_path}/{user}/{repo}`

**Custom libssh Wrapper (src/libssh/):**
- Wraps `libssh-rs-sys` with vendored libssh
- `Listener`: Binds SSH server with host key
- `Session`: Handles SSH session lifecycle
- `Channel`: Bidirectional data transfer with stderr flag
- Event-driven architecture via `ChannelEvent` enum

**Frontend Stack:**
- Server-side rendering with Maud (type-safe HTML in Rust)
- Tailwind CSS for styling
- HTMX for dynamic interactions
- Ace Editor for code editing
- Auto-reload in debug builds

**Version Display:**
Version printed on startup: `conduit dev-{hash}[-dirty]`

## Important Notes

- Repository paths are NOT validated before spawning Git processes
- SSH key authentication is stubbed (empty vec) - needs database loading
- No HTTPS support - designed for reverse proxy deployment
- Session cookies: SameSite=Lax, HTTP-only (details in middleware)
- Password hashing uses SHA256 (consider stronger algorithm for production)
- Single-user or small-team focused - not multi-tenant
