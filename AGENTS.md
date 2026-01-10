# Repository Guidelines

## Project Structure & Module Organization
- `src/` contains the Rust application entrypoint (`main.rs`) and core modules.
- `src/routes/` holds HTTP route groups; `src/model/` encapsulates SQLx data access; `src/middleware/` contains request middleware.
- `src/libssh/` and `src/ssh.rs` implement the SSH/Git server side.
- `public/` serves static assets (HTMX, Ace, etc.); `styles/` is the Tailwind input.
- `migrations/` stores database migrations; `data/` is runtime state (Git repositories, SSH host key).
- `config.example.toml` shows the expected configuration schema.

## Build, Test, and Development Commands
- `cargo build --release`: build a release binary; `build.rs` also runs Tailwind via the `tailwindcss` CLI.
- `cargo run`: run the combined HTTP (8080) + SSH (8022) servers locally.
- `cargo test`: run the Rust test suite (currently minimal).
- `cargo fmt` / `cargo clippy`: format and lint using the repo’s Rustfmt config.
- `docker compose up -d postgres`: start a local PostgreSQL instance for development.

Prereqs: Rust nightly `nightly-2025-08-01`, PostgreSQL, `tailwindcss` in `PATH`, and Git binaries (`git-upload-pack`, `git-receive-pack`).

## Coding Style & Naming Conventions
- Rust edition is 2024; follow standard Rust formatting (4-space indents) with `rustfmt.toml` settings.
- Use `snake_case` for modules/functions and `PascalCase` for types; keep route handlers small and composable.

## Testing Guidelines
- Prefer unit tests with `#[test] fn descriptive_name()`; integration tests can live under `tests/` if added.
- No explicit coverage target is documented; add tests alongside behavior changes when possible.

## Commit & Pull Request Guidelines
- Commit messages are short and lowercase, topic-first (examples: `paste acl`, `profile configuration`).
- PRs should include a clear description, the commands run, and screenshots for UI changes.
- Schema changes should include a matching SQL migration in `migrations/`.

## Configuration & Security Notes
- Copy `config.example.toml` to `config.toml`; `.env` is auto-loaded and `DATABASE_URL` overrides config DB settings.
- Keep secrets out of git; the SSH host key should live in `data/` per config.
