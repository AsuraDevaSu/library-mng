use leptos::prelude::*;

use crate::{
    auth::err_msg,
    server_fns::search_books,
    ui::{Alert, AlertVariant, Badge, BadgeVariant, Card, Highlight, INPUT, Tone, cn},
};

#[component]
pub fn BooksPage() -> impl IntoView {
    let (query, set_query) = signal(String::new());
    let books = Resource::new(move || query.get(), search_books);

    view! {
        <section class="py-8">
            <div class="mb-8 flex flex-col gap-4 md:flex-row md:items-end md:justify-between">
                <div>
                    <h1 class="mb-3 text-4xl font-medium md:text-5xl"><Highlight>"Catalog"</Highlight></h1>
                    <p class="text-lg">"Search by title, author or ISBN. Badges show copies on the shelf right now."</p>
                </div>
            </div>

            <div class="relative mb-10">
                <svg class="pointer-events-none absolute top-1/2 left-5 size-5 -translate-y-1/2" viewBox="0 0 24 24"
                    fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" aria-hidden="true">
                    <circle cx="11" cy="11" r="7"/><path d="m20 20-4-4"/>
                </svg>
                <input
                    class=cn(&[INPUT, "h-16 rounded-2xl pl-14 text-lg shadow-brut-sm"])
                    type="search"
                    aria-label="Search the catalog"
                    placeholder="Try “clean code”, “Martin” or an ISBN"
                    prop:value=query
                    on:input=move |ev| set_query.set(event_target_value(&ev))
                />
            </div>

            <Transition fallback=|| view! { <p class="text-muted-foreground">"Loading…"</p> }>
                {move || Suspend::new(async move {
                    match books.await {
                        Err(e) => view! { <Alert variant=AlertVariant::Destructive>{err_msg(&e)}</Alert> }.into_any(),
                        Ok(list) if list.is_empty() => view! {
                            <Card tone=Tone::Muted class="p-10 text-center">
                                <p class="text-xl font-medium">"No books found"</p>
                                <p class="mt-2 text-muted-foreground">"Try a shorter search, or ask a librarian at the front desk."</p>
                            </Card>
                        }
                        .into_any(),
                        Ok(list) => view! {
                            <div class="grid gap-6 sm:grid-cols-2 lg:grid-cols-3">
                                {list
                                    .into_iter()
                                    .map(|b| {
                                        let available = b.available_copies > 0;
                                        view! {
                                            <Card tone={if available { Tone::Plain } else { Tone::Muted }} class="flex flex-col gap-4 p-6">
                                                <div class="flex items-start justify-between gap-3">
                                                    <h2 class="text-xl leading-snug font-medium">{b.title}</h2>
                                                    <Badge variant={if available { BadgeVariant::Brand } else { BadgeVariant::Muted }}>
                                                        {if available {
                                                            format!("{} of {} available", b.available_copies, b.total_copies)
                                                        } else {
                                                            "All on loan".to_string()
                                                        }}
                                                    </Badge>
                                                </div>
                                                <p class="text-muted-foreground">
                                                    {if b.authors.is_empty() { "Unknown author".to_string() } else { b.authors }}
                                                </p>
                                                <div class="mt-auto flex flex-wrap gap-x-4 gap-y-1 border-t border-hairline pt-3 text-sm text-muted-foreground">
                                                    {b.published_year.map(|y| view! { <span>{y.to_string()}</span> })}
                                                    {b.isbn.map(|i| view! { <span class="font-mono">"ISBN " {i}</span> })}
                                                </div>
                                            </Card>
                                        }
                                    })
                                    .collect_view()}
                            </div>
                        }
                        .into_any(),
                    }
                })}
            </Transition>
        </section>
    }
}
