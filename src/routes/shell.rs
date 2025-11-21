use crate::auth::Session;

pub fn document<S: Into<Option<Session>>>(
    markup: maud::Markup,
    title: &str,
    session: S,
) -> maud::Markup {
    let session = session.into();

    maud::html! {
        (maud::DOCTYPE)
        html lang="en" {
            head {
                meta charset="UTF-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                link rel="stylesheet" href="/assets/index.css";
                (scripts())
                title { (title) " - conduit" }
            }

            body {
                (header(&session))
                main .container .m-auto .px-50 .m-auto { (markup) }
            }
        }
    }
}

fn scripts() -> maud::Markup {
    #[cfg(debug_assertions)]
    maud::html! {
        script src="/assets/htmx-2.0.8.js" {}
        script src="/assets/autoreload.js" {}
    }
    #[cfg(not(debug_assertions))]
    maud::html! {
        script src="/assets/htmx-2.0.8.min.js" {}
    }
}

fn header(session: &Option<Session>) -> maud::Markup {
    maud::html! {
        nav .container .m-auto .px-50 .mb-4 .flex .justify-between {
            span {
                a .hover:underline href="/" { "conduit" }
            }
            @if let Some(session) = session {
                ul .flex .grow .ms-12 .gap-8 {
                    li { a .hover:underline href="/paste" { "paste" } }
                    li { a .hover:underline href="/meta" { "meta" } }
                }

                div {
                    span {
                        "Logged in as "
                        a .underline href={"/~" (session.username)} { (session.username) }
                        " - "
                        a .underline href="/logout" { "Log out" }
                    }
                }
            } @else {
                div {
                    span {
                        a .underline href="/login" { "Log in" }
                        " - "
                        a .underline href="/register" { "Register" }
                    }
                }
            }
        }
    }
}
