use axum::Router;
use axum::extract::Form;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use serde::Deserialize;

use crate::middleware::auth::Session;
use crate::model;
use crate::routes::{AppError, shell};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/meta/keys", get(page_keys))
        .route("/meta/keys", post(do_add_key))
        .route("/meta/keys/delete", post(do_delete_key))
}

async fn page_keys(state: AppState, session: Session) -> Result<Response, AppError> {
    let keys = model::user::get_user_keys(&state.db, session.id).await?;

    let markup = maud::html! {
        (super::meta_nav())

        h2 .text-xl .mt-4 .mb-2 { "SSH Keys" }

        @if keys.is_empty() {
            p .text-gray-600 .mb-4 { "No SSH keys configured." }
        } @else {
            div .mb-4 {
                @for key in &keys {
                    div .border-solid .border-1 .border-gray-300 .p-2 .mb-2 .flex .justify-between .items-start {
                        div .flex-1 .overflow-hidden {
                            div .font-semibold .mb-1 {
                                (key.name)
                            }
                            div .font-mono .text-sm {
                                span .text-gray-600 { (key.key_type) " " }
                                span .break-all { (truncate_key(&key.encoded)) }
                            }
                            div .text-sm .text-gray-600 .mt-1 {
                                (key.username) "@" (key.hostname)
                            }
                        }
                        form method="post" action="/meta/keys/delete" .ml-2 {
                            input type="hidden" name="key_type" value=(key.key_type);
                            input type="hidden" name="encoded" value=(key.encoded);
                            button
                                .text-red-600
                                .hover:underline
                                .text-sm
                                type="submit"
                            {
                                "delete"
                            }
                        }
                    }
                }
            }
        }

        h3 .text-lg .mt-6 .mb-2 { "Add SSH Key" }
        form method="post" {
            div .mb-2 {
                label for="name" .block .mb-1 { "Name" }
                input
                    .border-solid
                    .border-1
                    .border-gray-300
                    .w-full
                    .p-2
                    type="text"
                    name="name"
                    placeholder="e.g., Laptop, Work Computer"
                    required;
            }
            div .mb-2 {
                label for="pubkey" .block .mb-1 { "Public Key" }
                textarea
                    .border-solid
                    .border-1
                    .border-gray-300
                    .w-full
                    .font-mono
                    .text-sm
                    .p-2
                    name="pubkey"
                    rows="3"
                    placeholder="ssh-ed25519 AAAAC3... user@hostname"
                    required {}
            }
            p .text-sm .text-gray-600 .mb-3 {
                "Paste your public SSH key. Only ssh-ed25519 keys are supported."
            }
            input
                .text-neutral-50
                .bg-blue-500
                .border-neutral-700
                .border-solid
                .border-1
                .px-3
                .py-1
                type="submit"
                value="Add Key";
        }
    };

    Ok(shell::document(markup, "keys", session).into_response())
}

fn truncate_key(key: &str) -> String {
    if key.len() > 40 {
        format!("{}...", &key[..40])
    } else {
        key.to_string()
    }
}

#[derive(Deserialize)]
struct AddKeyForm {
    name: String,
    pubkey: String,
}

async fn do_add_key(
    state: AppState,
    session: Session,
    Form(form): Form<AddKeyForm>,
) -> Result<Redirect, AppError> {
    let pubkey = form.pubkey.trim();
    let name = form.name.trim();

    // Parse SSH key format: "ssh-ed25519 AAAAC3... user@hostname"
    let parts: Vec<&str> = pubkey.split_whitespace().collect();

    if parts.len() < 2 {
        return Err(anyhow::anyhow!(
            "Invalid SSH key format. Expected: ssh-ed25519 <key> [comment]"
        )
        .into());
    }

    let key_type = parts[0];
    let encoded = parts[1];

    // Validate key type
    if key_type != "ssh-ed25519" {
        return Err(anyhow::anyhow!("Only ssh-ed25519 keys are supported").into());
    }

    // Parse comment (username@hostname) or use defaults
    let (username, hostname) = if parts.len() >= 3 {
        let comment = parts[2];
        if let Some(at_pos) = comment.find('@') {
            let user = &comment[..at_pos];
            let host = &comment[at_pos + 1..];
            (user.to_string(), host.to_string())
        } else {
            (comment.to_string(), "unknown".to_string())
        }
    } else {
        ("unknown".to_string(), "unknown".to_string())
    };

    model::user::add_user_key(
        &state.db, session.id, key_type, encoded, &username, &hostname, name,
    )
    .await?;

    Ok(Redirect::to("/meta/keys"))
}

#[derive(Deserialize)]
struct DeleteKeyForm {
    key_type: String,
    encoded: String,
}

async fn do_delete_key(
    state: AppState,
    _session: Session,
    Form(form): Form<DeleteKeyForm>,
) -> Result<Redirect, AppError> {
    model::user::delete_user_key(&state.db, &form.key_type, &form.encoded).await?;
    Ok(Redirect::to("/meta/keys"))
}
