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
                main .container { (markup) }
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
        nav .container .navbar .navbar-expand {
            span {
                a href="/" { "conduit" }
            }
            @if let Some(session) = session {
                ul .nav .nav-top .navbar-expand {
                    li { a href="/paste" { "paste" } }
                    li { a href="/meta" { "meta" } }
                }

                div .navbar-right {
                    span {
                        "Logged in as "
                        a href={"/~" (session.username)} { (session.username) }
                        " - "
                        a href="/logout" { "Log out" }
                    }
                }
            } @else {
                div .navbar-right {
                    span {
                        a href="/login" { "Log in" }
                        " - "
                        a href="/register" { "Register" }
                    }
                }
            }
        }
    }
}
