//! Public catalog: search, book page and the online reader. No login needed;
//! students and teachers browse what the library has and read e-books.

use domain::models::{BookSummary, CopyStatus, ebook_url};
use leptos::prelude::*;
use leptos_router::{
    NavigateOptions,
    hooks::{use_navigate, use_params_map, use_query_map},
};
use uuid::Uuid;

use crate::{
    auth::err_msg,
    server_fns::{book_categories, get_book, search_books},
    ui::{Alert, AlertVariant, Badge, BadgeVariant, ButtonLink, Card, Highlight, INPUT, Size, Tone, Variant, cn},
};

/// Cover image, or a coloured placeholder with the title.
#[component]
pub fn Cover(
    #[prop(into)] title: String,
    url: Option<String>,
    #[prop(optional, into)] class: String,
) -> impl IntoView {
    let class = cn(&["aspect-[2/3] w-full overflow-hidden rounded-xl border border-border", &class]);
    match url {
        Some(src) => view! {
            <img src=src alt=format!("Cover of {title}") loading="lazy" class=cn(&[&class, "bg-muted object-cover"])/>
        }
        .into_any(),
        None => view! {
            <div class=cn(&[&class, "flex flex-col justify-between bg-primary p-4 text-primary-foreground"]) aria-hidden="true">
                <span class="h-1.5 w-10 rounded-full bg-accent"></span>
                <span class="line-clamp-4 text-lg leading-snug font-medium">{title}</span>
            </div>
        }
        .into_any(),
    }
}

fn availability(b: &BookSummary) -> impl IntoView + use<> {
    let (variant, text) = if b.total_copies == 0 {
        (BadgeVariant::Muted, "Online only".to_string())
    } else if b.available_copies > 0 {
        (BadgeVariant::Brand, format!("{} of {} on the shelf", b.available_copies, b.total_copies))
    } else {
        (BadgeVariant::Muted, "All on loan".to_string())
    };
    // Online-only books already say so in the "Read online" badge.
    (b.total_copies > 0 || !b.has_ebook).then(|| view! { <Badge variant=variant>{text}</Badge> })
}

#[component]
pub fn BooksPage() -> impl IntoView {
    let query = use_query_map();
    let navigate = use_navigate();
    // Filters live in the URL so they can be linked to (e.g. "/books?online=1").
    let q = move || query.read().get("q").unwrap_or_default();
    let category = move || query.read().get("category").unwrap_or_default();
    let online = move || query.read().get("online").is_some_and(|v| v == "1");

    let filters = Callback::new(move |(q, category, online): (String, String, bool)| {
        let mut parts = Vec::new();
        if !q.is_empty() {
            parts.push(format!("q={}", urlencode(&q)));
        }
        if !category.is_empty() {
            parts.push(format!("category={}", urlencode(&category)));
        }
        if online {
            parts.push("online=1".into());
        }
        let url = if parts.is_empty() { "/books".to_string() } else { format!("/books?{}", parts.join("&")) };
        navigate(&url, NavigateOptions { replace: true, scroll: false, ..Default::default() });
    });
    let set_filters = move |q: String, category: String, online: bool| filters.run((q, category, online));

    let books = Resource::new(move || (q(), category(), online()), |(q, c, o)| search_books(q, c, o));
    let categories = Resource::new(|| (), |_| book_categories());

    let chip = "rounded-full border border-border px-4 py-1.5 text-sm transition-colors hover:bg-accent \
                aria-pressed:bg-primary aria-pressed:text-primary-foreground";

    view! {
        <section class="py-8">
            <div class="mb-8">
                <h1 class="mb-3 text-4xl font-medium md:text-5xl"><Highlight>"Catalog"</Highlight></h1>
                <p class="text-lg">"Every book in the library. Search by title, author or ISBN — some books you can read online right now."</p>
            </div>

            <div class="relative mb-5">
                <svg class="pointer-events-none absolute top-1/2 left-5 size-5 -translate-y-1/2" viewBox="0 0 24 24"
                    fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" aria-hidden="true">
                    <circle cx="11" cy="11" r="7"/><path d="m20 20-4-4"/>
                </svg>
                <input
                    class=cn(&[INPUT, "h-16 rounded-2xl pl-14 text-lg shadow-brut-sm"])
                    type="search"
                    aria-label="Search the catalog"
                    placeholder="Try “clean code”, “Martin” or an ISBN"
                    prop:value=q
                    on:input=move |ev| set_filters(event_target_value(&ev), category(), online())
                />
            </div>

            <div class="mb-10 flex flex-wrap items-center gap-2">
                <button type="button" class=chip aria-pressed=move || online().to_string()
                    on:click=move |_| set_filters(q(), category(), !online())>
                    "📖 Read online"
                </button>
                <span class="mx-2 h-6 w-px bg-border" aria-hidden="true"></span>
                <button type="button" class=chip aria-pressed=move || category().is_empty().to_string()
                    on:click=move |_| set_filters(q(), String::new(), online())>
                    "All subjects"
                </button>
                <Transition fallback=|| ()>
                    {move || Suspend::new(async move {
                        categories.await.unwrap_or_default().into_iter().map(|c| {
                            let c2 = c.clone();
                            let c3 = c.clone();
                            view! {
                                <button type="button" class=chip aria-pressed=move || (category() == c2).to_string()
                                    on:click=move |_| set_filters(q(), c3.clone(), online())>
                                    {c}
                                </button>
                            }
                        }).collect_view()
                    })}
                </Transition>
            </div>

            <Transition fallback=|| view! { <p class="text-muted-foreground">"Loading…"</p> }>
                {move || Suspend::new(async move {
                    match books.await {
                        Err(e) => view! { <Alert variant=AlertVariant::Destructive>{err_msg(&e)}</Alert> }.into_any(),
                        Ok(list) if list.is_empty() => view! {
                            <Card tone=Tone::Muted class="p-10 text-center">
                                <p class="text-xl font-medium">"No books found"</p>
                                <p class="mt-2 text-muted-foreground">"Try a shorter search or another subject, or ask a librarian at the front desk."</p>
                            </Card>
                        }
                        .into_any(),
                        Ok(list) => view! {
                            <div class="grid grid-cols-2 gap-x-5 gap-y-8 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5">
                                {list.into_iter().map(|b| view! { <BookCard book=b/> }).collect_view()}
                            </div>
                        }
                        .into_any(),
                    }
                })}
            </Transition>
        </section>
    }
}

#[component]
fn BookCard(book: BookSummary) -> impl IntoView {
    let href = format!("/books/{}", book.id);
    let cover = book.cover_url();
    let badge = availability(&book);
    view! {
        <a href=href class="group flex flex-col gap-3 rounded-xl outline-none focus-visible:ring-4 focus-visible:ring-ring/70">
            <div class="relative transition-transform group-hover:-translate-y-1">
                <Cover title=book.title.clone() url=cover class="shadow-brut-sm"/>
                {book.has_ebook.then(|| view! {
                    <span class="absolute top-2 left-2 rounded-md bg-accent px-2 py-0.5 text-xs font-medium text-accent-foreground">
                        "Read online"
                    </span>
                })}
            </div>
            <div class="flex flex-col gap-1">
                <h2 class="line-clamp-2 leading-snug font-medium group-hover:underline">{book.title}</h2>
                <p class="line-clamp-1 text-sm text-muted-foreground">
                    {if book.authors.is_empty() { "Unknown author".to_string() } else { book.authors }}
                </p>
                <div class="mt-1">{badge}</div>
            </div>
        </a>
    }
}

fn book_id() -> impl Fn() -> Option<Uuid> + Copy + Send + Sync + 'static {
    let params = use_params_map();
    move || params.read().get("id").and_then(|s| s.parse().ok())
}

#[component]
pub fn BookPage() -> impl IntoView {
    let id = book_id();
    let detail = Resource::new(id, |id| async move {
        match id {
            Some(id) => get_book(id).await,
            None => Err(ServerFnError::new("book not found")),
        }
    });

    view! {
        <section class="py-8">
            <a href="/books" class="mb-6 inline-block text-muted-foreground hover:underline">"← Back to catalog"</a>
            <Transition fallback=|| view! { <p class="text-muted-foreground">"Loading…"</p> }>
                {move || Suspend::new(async move {
                    let d = match detail.await {
                        Ok(d) => d,
                        Err(e) => return view! { <Alert variant=AlertVariant::Destructive>{err_msg(&e)}</Alert> }.into_any(),
                    };
                    let b = d.book;
                    let total = d.copies.iter().filter(|c| c.status != CopyStatus::Withdrawn).count();
                    let available: Vec<_> = d.copies.iter().filter(|c| c.status == CopyStatus::Available).collect();
                    let mut shelves: Vec<String> = available.iter().filter_map(|c| c.shelf_location.clone()).collect();
                    shelves.sort();
                    shelves.dedup();
                    let n_available = available.len();
                    let read_href = format!("/books/{}/read", b.id);
                    let facts: Vec<(&str, String)> = [
                        ("Category", b.category.clone()),
                        ("Publisher", b.publisher.clone()),
                        ("Year", b.published_year.map(|y| y.to_string())),
                        ("Language", b.language.clone()),
                        ("ISBN", b.isbn.clone()),
                    ]
                    .into_iter()
                    .filter_map(|(k, v)| v.map(|v| (k, v)))
                    .collect();

                    view! {
                        <div class="grid gap-10 md:grid-cols-[minmax(0,280px)_minmax(0,1fr)]">
                            <div class="mx-auto w-full max-w-[280px]">
                                <Cover title=b.title.clone() url=b.cover_url() class="shadow-brut"/>
                            </div>
                            <div class="min-w-0">
                                {b.category.clone().map(|c| view! { <Badge variant=BadgeVariant::Muted>{c}</Badge> })}
                                <h1 class="mt-3 text-3xl font-medium md:text-5xl">{b.title.clone()}</h1>
                                {b.subtitle.clone().map(|s| view! { <p class="mt-2 text-xl text-muted-foreground">{s}</p> })}
                                <p class="mt-4 text-lg">
                                    {if b.authors.is_empty() { "Unknown author".to_string() } else { format!("by {}", b.authors) }}
                                </p>

                                <div class="mt-8 flex flex-wrap gap-4">
                                    {b.has_ebook().then(|| view! {
                                        <ButtonLink href=read_href.clone() variant=Variant::Brand size=Size::Lg>"📖 Read online"</ButtonLink>
                                    })}
                                </div>

                                <Card tone={if n_available > 0 { Tone::Muted } else { Tone::Plain }} class="mt-8 p-6">
                                    {if total == 0 {
                                        view! {
                                            <p class="text-lg font-medium">"No printed copy"</p>
                                            <p class="mt-1 text-muted-foreground">"This book is available to read online only."</p>
                                        }.into_any()
                                    } else if n_available > 0 {
                                        view! {
                                            <p class="text-lg font-medium">{format!("{n_available} of {total} copies on the shelf")}</p>
                                            <p class="mt-1 text-muted-foreground">
                                                {if shelves.is_empty() {
                                                    "Ask at the front desk to borrow it.".to_string()
                                                } else {
                                                    format!("Find it at: {}. Bring your student card to borrow it.", shelves.join(", "))
                                                }}
                                            </p>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <p class="text-lg font-medium">{format!("All {total} copies are on loan")}</p>
                                            <p class="mt-1 text-muted-foreground">"Ask a librarian to reserve the next one for you."</p>
                                        }.into_any()
                                    }}
                                </Card>

                                {b.description.clone().map(|d| view! {
                                    <div class="mt-8">
                                        <h2 class="mb-2 text-xl font-medium">"About this book"</h2>
                                        <p class="leading-relaxed whitespace-pre-line">{d}</p>
                                    </div>
                                })}

                                {(!facts.is_empty()).then(|| view! {
                                    <dl class="mt-8 grid grid-cols-[auto_1fr] gap-x-8 gap-y-2 border-t border-hairline pt-6">
                                        {facts.into_iter().map(|(k, v)| view! {
                                            <dt class="text-muted-foreground">{k}</dt>
                                            <dd class=if k == "ISBN" { "font-mono" } else { "" }>{v}</dd>
                                        }).collect_view()}
                                    </dl>
                                })}
                            </div>
                        </div>
                    }
                    .into_any()
                })}
            </Transition>
        </section>
    }
}

/// Full-screen online reader using the browser's built-in PDF viewer.
#[component]
pub fn ReaderPage() -> impl IntoView {
    let id = book_id();
    let detail = Resource::new(id, |id| async move {
        match id {
            Some(id) => get_book(id).await,
            None => Err(ServerFnError::new("book not found")),
        }
    });

    view! {
        <Transition fallback=|| view! { <p class="p-8 text-muted-foreground">"Loading…"</p> }>
            {move || Suspend::new(async move {
                match detail.await {
                    Ok(d) if d.book.has_ebook() => {
                        let pdf = ebook_url(d.book.id);
                        let back = format!("/books/{}", d.book.id);
                        view! {
                            <div class="flex h-dvh flex-col bg-background">
                                <div class="flex items-center gap-4 border-b border-border px-4 py-3">
                                    <a href=back class="shrink-0 hover:underline">"← Back"</a>
                                    <h1 class="min-w-0 flex-1 truncate font-medium">{d.book.title}</h1>
                                    <a href=pdf.clone() target="_blank" rel="noopener"
                                        class=crate::ui::button_class(Variant::Outline, Size::Sm)>
                                        "Open full screen"
                                    </a>
                                </div>
                                <iframe src=pdf title="Book reader" class="w-full flex-1 border-0"></iframe>
                            </div>
                        }
                        .into_any()
                    }
                    Ok(_) => view! {
                        <div class="mx-auto max-w-xl p-8">
                            <Alert>"This book can't be read online. Borrow a printed copy at the library."</Alert>
                        </div>
                    }
                    .into_any(),
                    Err(e) => view! {
                        <div class="mx-auto max-w-xl p-8">
                            <Alert variant=AlertVariant::Destructive>{err_msg(&e)}</Alert>
                        </div>
                    }
                    .into_any(),
                }
            })}
        </Transition>
    }
}

/// Minimal percent-encoding for query values.
fn urlencode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}
