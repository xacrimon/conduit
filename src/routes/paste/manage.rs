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
        .route("/paste/manage", get(page_manage))
        .route("/paste/manage/delete", post(do_delete_paste))
}

async fn page_manage(state: AppState, session: Session) -> Result<Response, AppError> {
    let pastes = model::paste::get_user_pastes(&state.db, session.id).await?;

    let markup = maud::html! {
        h2 .text-xl .mb-4 { "Your Pastes" }

        @if pastes.is_empty() {
            p .text-gray-600 .mb-4 { "No pastes yet. " a .underline href="/paste" { "Create one" } "." }
        } @else {
            div .mb-4 {
                @for paste in &pastes {
                    div .border-solid .border-1 .border-gray-300 .p-3 .mb-2 .flex .justify-between .items-center {
                        div .flex-1 {
                            a .font-mono .text-blue-600 .hover:underline href=(format!("/~{}/paste/{}", session.username, paste.id)) {
                                (paste.filename)
                            }
                            span .text-gray-500 .text-sm .ml-3 {
                                (paste.id)
                            }
                            span .ml-3 {
                                @if paste.visibility == "public" {
                                    span .text-xs .bg-green-100 .text-green-800 .px-2 .py-1 .rounded { "public" }
                                } @else if paste.visibility == "unlisted" {
                                    span .text-xs .bg-yellow-100 .text-yellow-800 .px-2 .py-1 .rounded { "unlisted" }
                                } @else {
                                    span .text-xs .bg-gray-100 .text-gray-800 .px-2 .py-1 .rounded { "private" }
                                }
                            }
                        }
                        form method="post" action="/paste/manage/delete" .ml-2 {
                            input type="hidden" name="paste_id" value=(paste.id);
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

        div .mt-6 {
            a
                .text-neutral-50
                .bg-blue-500
                .hover:bg-blue-600
                .border-neutral-700
                .border-solid
                .border-1
                .px-3
                .py-1
                .no-underline
                .inline-block
                href="/paste"
            {
                "New Paste"
            }
        }
    };

    Ok(shell::document(markup, "manage pastes", session).into_response())
}

#[derive(Deserialize)]
struct DeletePasteForm {
    paste_id: String,
}

async fn do_delete_paste(
    state: AppState,
    session: Session,
    Form(form): Form<DeletePasteForm>,
) -> Result<Redirect, AppError> {
    model::paste::delete_paste(&state.db, session.id, &form.paste_id).await?;
    Ok(Redirect::to("/paste/manage"))
}
