use crate::middleware::auth::Session;
use crate::routes::assets;

pub fn document<S: Into<Option<Session>>>(
    markup: maud::Markup,
    title: &str,
    session: S,
) -> maud::Markup {
    document_with(markup, title, session, maud::html! {})
}

pub fn document_with<S: Into<Option<Session>>>(
    markup: maud::Markup,
    title: &str,
    session: S,
    extra: maud::Markup,
) -> maud::Markup {
    let session = session.into();
    document_impl(markup, title, session, extra)
}

fn document_impl(
    markup: maud::Markup,
    title: &str,
    session: Option<Session>,
    extra: maud::Markup,
) -> maud::Markup {
    maud::html! {
        (maud::DOCTYPE)
        html lang="en" {
            head {
                meta charset="UTF-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                link rel="stylesheet" href={ "/assets/" (assets::CSS_ASSET_NAME) };
                (scripts())
                (extra)
                title { (title) " - conduit" }
            }

            body {
                div .container .m-auto .2xl:px-50 .xl:px-20 .lg:px-12 .md:px-4 .sm:px-2 {
                    (header(&session))
                    main { (markup) }
                }
            }
        }
    }
}

fn scripts() -> maud::Markup {
    maud::html! {
        script src="/assets/lib/htmx-2.0.8.js" {}
        @if cfg!(debug_assertions) {
            script src=(assets::path("autoreload.js")) {}
        }
    }
}

pub fn subnav(items: &[(&str, &str)], current: &str) -> maud::Markup {
    maud::html! {
        div .border-b .border-gray-300 .mb-3 {
            ul .flex .gap-1 .text-sm {
                @for (name, href) in items {
                    @if *name == current {
                        li {
                            a
                                .block
                                .px-2
                                .py-1
                                .bg-gray-200
                                .text-black
                                .border
                                .border-gray-300
                                href=(*href)
                            {
                                (*name)
                            }
                        }
                    } @else {
                        li {
                            a
                                .block
                                .px-2
                                .py-1
                                .text-gray-600
                                .hover:text-black
                                .hover:bg-gray-100
                                .border
                                .border-transparent
                                href=(*href)
                            {
                                (*name)
                            }
                        }
                    }
                }
            }
        }
    }
}

fn header(session: &Option<Session>) -> maud::Markup {
    maud::html! {
        nav .mb-4 .flex .justify-between {
            span {
                a .hover:underline href="/" { "conduit" }
            }
            @if let Some(session) = session {
                ul .flex .grow .ms-12 .gap-8 {
                    li { a .text-gray-500 .hover:text-gray-700 href="/paste" { "paste" } }
                    li { a .text-gray-500 .hover:text-gray-700 href="/meta" { "meta" } }
                }

                div {
                    span {
                        "Logged in as "
                        a .underline href={ "/~" (session.username)} { (session.username) }
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
