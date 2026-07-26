mod blob;
mod tree;

use axum::Router;

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().merge(tree::routes()).merge(blob::routes())
}

pub fn breadcrumbs(username: &str, repo_name: &str, path: &str) -> maud::Markup {
    let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    maud::html! {
        div .mb-3 .text-sm {
            a .text-blue-600 .hover:underline href=(format!("/~{}/{}", username, repo_name)) { (repo_name) }
            @for (i, part) in parts.iter().enumerate() {
                " / "
                @if i == parts.len() - 1 {
                    span .font-semibold { (part) }
                } @else {
                    @let sub_path = parts[..=i].join("/");
                    a .text-blue-600 .hover:underline href=(format!("/~{}/{}/tree/{}", username, repo_name, sub_path)) {
                        (part)
                    }
                }
            }
        }
    }
}
