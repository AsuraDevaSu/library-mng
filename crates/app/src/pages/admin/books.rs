//! Book management for librarians and admins: create / edit books, add
//! copies, upload a cover and an e-book (PDF) so students can read online.

use domain::{
    dto::BookForm,
    models::{Book, CopyStatus},
};
use leptos::{html, prelude::*};
use leptos_router::hooks::{use_params_map, use_query_map};
use uuid::Uuid;

use super::PageTitle;
use crate::{
    admin_fns::{AddBookCopies, CreateBook, DeleteBook, UpdateBook, admin_book, admin_books},
    auth::err_msg,
    pages::books::Cover,
    ui::{
        Alert, AlertVariant, Badge, BadgeVariant, Button, ButtonLink, Card, CardContent, CardDescription, CardHeader,
        CardTitle, INPUT, LABEL, Size, TD, TH, TR, Table, TableBody, TableHeader, Tone, Variant, cn,
    },
};

#[component]
pub fn AdminBooksPage() -> impl IntoView {
    let (q, set_q) = signal(String::new());
    let books = Resource::new(move || q.get(), admin_books);

    view! {
        <div class="mb-8 flex flex-col gap-4 md:flex-row md:items-end md:justify-between">
            <PageTitle title="Books" subtitle="What students and teachers see in the catalog."/>
            <ButtonLink href="/admin/books/new" variant=Variant::Brand class="mb-8">"+ New book"</ButtonLink>
        </div>
        <input
            class=cn(&[INPUT, "mb-6 max-w-md"])
            type="search"
            aria-label="Search books"
            placeholder="Search title, author or ISBN"
            prop:value=q
            on:input=move |ev| set_q.set(event_target_value(&ev))
        />
        <Transition fallback=|| view! { <p class="text-muted-foreground">"Loading…"</p> }>
            {move || Suspend::new(async move {
                match books.await {
                    Err(e) => view! { <Alert variant=AlertVariant::Destructive>{err_msg(&e)}</Alert> }.into_any(),
                    Ok(list) if list.is_empty() => view! {
                        <Card tone=Tone::Muted class="p-10 text-center">
                            <p class="text-xl font-medium">"No books yet"</p>
                            <p class="mt-2 text-muted-foreground">"Add the first one with “New book”."</p>
                        </Card>
                    }
                    .into_any(),
                    Ok(list) => view! {
                        <Table>
                            <TableHeader>
                                <tr>
                                    <th class=TH>"Book"</th>
                                    <th class=TH>"Category"</th>
                                    <th class=TH>"Copies"</th>
                                    <th class=TH>"Online"</th>
                                </tr>
                            </TableHeader>
                            <TableBody>
                                {list.into_iter().map(|b| {
                                    let href = format!("/admin/books/{}", b.id);
                                    let cover = b.cover_url();
                                    view! {
                                        <tr class=TR>
                                            <td class=TD>
                                                <a href=href class="flex items-center gap-3 hover:underline">
                                                    <div class="w-10 shrink-0"><Cover title=b.title.clone() url=cover class="rounded-md text-[0px]"/></div>
                                                    <div>
                                                        <div class="font-medium">{b.title}</div>
                                                        <div class="text-sm text-muted-foreground">{b.authors}</div>
                                                    </div>
                                                </a>
                                            </td>
                                            <td class=TD>{b.category.unwrap_or_default()}</td>
                                            <td class=cn(&[TD, "tabular-nums"])>{format!("{} / {}", b.available_copies, b.total_copies)}</td>
                                            <td class=TD>
                                                {if b.has_ebook {
                                                    view! { <Badge variant=BadgeVariant::Brand>"PDF"</Badge> }.into_any()
                                                } else {
                                                    view! { <span class="text-muted-foreground">"—"</span> }.into_any()
                                                }}
                                            </td>
                                        </tr>
                                    }
                                }).collect_view()}
                            </TableBody>
                        </Table>
                    }
                    .into_any(),
                }
            })}
        </Transition>
    }
}

/// Inputs shared by the create and edit forms (`form[...]` → `BookForm`).
#[component]
fn BookFields(initial: BookForm, #[prop(optional)] with_copies: bool) -> impl IntoView {
    view! {
        <div class="grid gap-4 md:grid-cols-2">
            <label class=cn(&[LABEL, "md:col-span-2"])>"Title *"
                <input class=INPUT name="form[title]" value=initial.title required=true maxlength="500"/>
            </label>
            <label class=cn(&[LABEL, "md:col-span-2"])>"Subtitle"
                <input class=INPUT name="form[subtitle]" value=initial.subtitle maxlength="500"/>
            </label>
            <label class=LABEL>"Authors * (separate with commas)"
                <input class=INPUT name="form[authors]" value=initial.authors required=true placeholder="e.g. Robert C. Martin"/>
            </label>
            <label class=LABEL>"Category / subject"
                <input class=INPUT name="form[category]" value=initial.category placeholder="e.g. Science, Khmer literature" maxlength="100"/>
            </label>
            <label class=LABEL>"ISBN"
                <input class=INPUT name="form[isbn]" value=initial.isbn placeholder="10 or 13 digits"/>
            </label>
            <label class=LABEL>"Publisher"
                <input class=INPUT name="form[publisher]" value=initial.publisher/>
            </label>
            <label class=LABEL>"Year published"
                <input class=INPUT name="form[published_year]" value=initial.published_year inputmode="numeric" pattern="[0-9]{4}" placeholder="2019"/>
            </label>
            <label class=LABEL>"Language"
                <input class=INPUT name="form[language]" value=initial.language placeholder="e.g. Khmer, English"/>
            </label>
            <label class=cn(&[LABEL, "md:col-span-2"])>"Description"
                <textarea class=cn(&[INPUT, "h-auto min-h-28 py-3"]) name="form[description]" rows="4">{initial.description}</textarea>
            </label>
            {with_copies.then(|| view! {
                <label class=LABEL>"Copy barcodes (optional)"
                    <input class=INPUT name="form[copy_barcodes]" placeholder="BK-0001, BK-0002"/>
                    <span class="text-xs font-normal text-muted-foreground">"One per printed copy. Leave empty for an online-only book."</span>
                </label>
                <label class=LABEL>"Shelf location"
                    <input class=INPUT name="form[shelf_location]" placeholder="e.g. Shelf A3"/>
                </label>
            })}
        </div>
    }
}

#[component]
pub fn NewBookPage() -> impl IntoView {
    let create = ServerAction::<CreateBook>::new();
    view! {
        <a href="/admin/books" class="mb-4 inline-block text-muted-foreground hover:underline">"← All books"</a>
        <PageTitle title="New book" subtitle="After saving you can add a cover and a PDF for reading online."/>
        <Card>
            <CardContent class="pt-6">
                {move || create.value().get().and_then(|r| r.err()).map(|e| view! {
                    <Alert variant=AlertVariant::Destructive>{err_msg(&e)}</Alert>
                })}
                <ActionForm action=create attr:class="flex flex-col gap-6">
                    <BookFields initial=BookForm::default() with_copies=true/>
                    <div>
                        <Button kind="submit" variant=Variant::Brand disabled=Signal::derive(move || create.pending().get())>
                            "Save book"
                        </Button>
                    </div>
                </ActionForm>
            </CardContent>
        </Card>
    }
}

fn to_form(b: &Book) -> BookForm {
    BookForm {
        title: b.title.clone(),
        subtitle: b.subtitle.clone().unwrap_or_default(),
        authors: b.authors.clone(),
        isbn: b.isbn.clone().unwrap_or_default(),
        category: b.category.clone().unwrap_or_default(),
        publisher: b.publisher.clone().unwrap_or_default(),
        published_year: b.published_year.map(|y| y.to_string()).unwrap_or_default(),
        language: b.language.clone().unwrap_or_default(),
        description: b.description.clone().unwrap_or_default(),
        ..Default::default()
    }
}

fn status_label(s: CopyStatus) -> &'static str {
    match s {
        CopyStatus::Available => "On shelf",
        CopyStatus::OnLoan => "On loan",
        CopyStatus::OnHoldShelf => "On hold shelf",
        CopyStatus::Lost => "Lost",
        CopyStatus::Damaged => "Damaged",
        CopyStatus::Withdrawn => "Withdrawn",
    }
}

#[component]
pub fn EditBookPage() -> impl IntoView {
    let params = use_params_map();
    let query = use_query_map();
    let id = move || params.read().get("id").and_then(|s| s.parse::<Uuid>().ok());

    let update = ServerAction::<UpdateBook>::new();
    let add_copies = ServerAction::<AddBookCopies>::new();
    let delete = ServerAction::<DeleteBook>::new();
    // Bumped after a cover / PDF upload so the page reloads the book.
    let (files_version, set_files_version) = signal(0u32);
    let refresh = Callback::new(move |_: ()| set_files_version.update(|v| *v += 1));

    let detail = Resource::new(
        move || (id(), update.version().get(), add_copies.version().get(), files_version.get()),
        |(id, ..)| async move {
            match id {
                Some(id) => admin_book(id).await,
                None => Err(ServerFnError::new("book not found")),
            }
        },
    );

    view! {
        <a href="/admin/books" class="mb-4 inline-block text-muted-foreground hover:underline">"← All books"</a>
        {move || query.read().get("created").map(|_| view! {
            <Alert variant=AlertVariant::Success>"Book created. Add a cover or a PDF below if you have one."</Alert>
        })}
        <Transition fallback=|| view! { <p class="text-muted-foreground">"Loading…"</p> }>
            {move || Suspend::new(async move {
                let d = match detail.await {
                    Ok(d) => d,
                    Err(e) => return view! { <Alert variant=AlertVariant::Destructive>{err_msg(&e)}</Alert> }.into_any(),
                };
                let b = d.book;
                let book_id = b.id;
                let public_href = format!("/books/{book_id}");
                let form = to_form(&b);
                let ebook_size = b.ebook_bytes.map(|n| format!("{:.1} MB", n as f64 / 1_048_576.0));
                let (title, cover_url, has_cover, has_ebook) = (b.title.clone(), b.cover_url(), b.has_cover, b.has_ebook());
                view! {
                    <div class="mb-8 flex flex-col gap-3 md:flex-row md:items-end md:justify-between">
                        <PageTitle title=b.title.clone() subtitle=b.authors.clone()/>
                        <a href=public_href target="_blank" rel="noopener" class=cn(&[&crate::ui::button_class(Variant::Outline, Size::Sm), "mb-8"])>
                            "View in public catalog ↗"
                        </a>
                    </div>

                    <div class="grid items-start gap-6 xl:grid-cols-[minmax(0,1fr)_340px]">
                        <div class="flex min-w-0 flex-col gap-6">
                            <Card>
                                <CardHeader><CardTitle>"Details"</CardTitle></CardHeader>
                                <CardContent>
                                    {outcome!(update, "Saved.")}
                                    <ActionForm action=update attr:class="flex flex-col gap-6">
                                        <input type="hidden" name="id" value=book_id.to_string()/>
                                        <BookFields initial=form/>
                                        <div>
                                            <Button kind="submit" variant=Variant::Brand disabled=Signal::derive(move || update.pending().get())>
                                                "Save changes"
                                            </Button>
                                        </div>
                                    </ActionForm>
                                </CardContent>
                            </Card>

                            <Card>
                                <CardHeader>
                                    <CardTitle>"Printed copies"</CardTitle>
                                    <CardDescription>"Each physical copy has its own barcode."</CardDescription>
                                </CardHeader>
                                <CardContent>
                                    {if d.copies.is_empty() {
                                        view! { <p class="mb-4 text-muted-foreground">"No printed copies — online only."</p> }.into_any()
                                    } else {
                                        view! {
                                            <Table class="mb-6">
                                                <TableHeader>
                                                    <tr><th class=TH>"Barcode"</th><th class=TH>"Shelf"</th><th class=TH>"Status"</th></tr>
                                                </TableHeader>
                                                <TableBody>
                                                    {d.copies.into_iter().map(|c| view! {
                                                        <tr class=TR>
                                                            <td class=cn(&[TD, "font-mono"])>{c.barcode}</td>
                                                            <td class=TD>{c.shelf_location.unwrap_or_default()}</td>
                                                            <td class=TD>{status_label(c.status)}</td>
                                                        </tr>
                                                    }).collect_view()}
                                                </TableBody>
                                            </Table>
                                        }.into_any()
                                    }}
                                    {outcome!(add_copies, "Copies added.")}
                                    <ActionForm action=add_copies attr:class="grid items-end gap-4 md:grid-cols-[1fr_200px_auto]">
                                        <input type="hidden" name="id" value=book_id.to_string()/>
                                        <label class=LABEL>"New barcodes" <input class=INPUT name="barcodes" required=true placeholder="BK-0003, BK-0004"/></label>
                                        <label class=LABEL>"Shelf" <input class=INPUT name="shelf_location" placeholder="Shelf A3"/></label>
                                        <Button kind="submit" variant=Variant::Outline>"Add copies"</Button>
                                    </ActionForm>
                                </CardContent>
                            </Card>
                        </div>

                        <div class="flex flex-col gap-6">
                            <Card>
                                <CardHeader>
                                    <CardTitle>"Cover"</CardTitle>
                                    <CardDescription>"JPEG, PNG or WebP, up to 5 MB."</CardDescription>
                                </CardHeader>
                                <CardContent class="flex flex-col gap-4">
                                    <div class="mx-auto w-40"><Cover title=title url=cover_url/></div>
                                    <FileUpload book_id kind="cover" accept="image/jpeg,image/png,image/webp"
                                        max_mb=5 has_file=has_cover on_change=refresh/>
                                </CardContent>
                            </Card>
                            <Card tone={if has_ebook { Tone::Brand } else { Tone::Plain }}>
                                <CardHeader>
                                    <CardTitle>"Read online (PDF)"</CardTitle>
                                    <CardDescription>
                                        {match ebook_size {
                                            Some(size) => format!("PDF uploaded ({size}). Anyone can read it in the catalog."),
                                            None => "Upload a PDF to let students and teachers read this book online. Up to 200 MB.".into(),
                                        }}
                                    </CardDescription>
                                </CardHeader>
                                <CardContent>
                                    <FileUpload book_id kind="ebook" accept="application/pdf,.pdf"
                                        max_mb=200 has_file=has_ebook on_change=refresh/>
                                </CardContent>
                            </Card>
                            <Card tone=Tone::Muted>
                                <CardHeader>
                                    <CardTitle>"Delete book"</CardTitle>
                                    <CardDescription>"Only for books that were never borrowed or reserved."</CardDescription>
                                </CardHeader>
                                <CardContent>
                                    {move || delete.value().get().and_then(|r| r.err()).map(|e| view! {
                                        <Alert variant=AlertVariant::Destructive>{err_msg(&e)}</Alert>
                                    })}
                                    <ActionForm action=delete
                                        on:submit=|ev| {
                                            if !confirm("Delete this book, its copies and files? This can't be undone.") {
                                                ev.prevent_default();
                                            }
                                        }>
                                        <input type="hidden" name="id" value=book_id.to_string()/>
                                        <Button kind="submit" variant=Variant::Outline>"Delete book"</Button>
                                    </ActionForm>
                                </CardContent>
                            </Card>
                        </div>
                    </div>
                }
                .into_any()
            })}
        </Transition>
    }
}

fn confirm(_msg: &str) -> bool {
    #[cfg(feature = "hydrate")]
    {
        web_sys::window().and_then(|w| w.confirm_with_message(_msg).ok()).unwrap_or(false)
    }
    #[cfg(not(feature = "hydrate"))]
    {
        true
    }
}

/// Uploads (or removes) a cover or PDF through `/api/v1/books/{id}/files/{kind}`.
/// Uses `fetch` with the staff session cookie; files are streamed to disk on the server.
#[component]
fn FileUpload(
    book_id: Uuid,
    kind: &'static str,
    accept: &'static str,
    max_mb: u32,
    has_file: bool,
    on_change: Callback<()>,
) -> impl IntoView {
    let input = NodeRef::<html::Input>::new();
    let (busy, set_busy) = signal(false);
    let (error, set_error) = signal(None::<String>);
    let (chosen, set_chosen) = signal(false);
    let url = format!("/api/v1/books/{book_id}/files/{kind}");
    let url2 = url.clone();

    let upload = move |_| {
        set_error.set(None);
        #[cfg(feature = "hydrate")]
        {
            let Some(file) = input.get_untracked().and_then(|i| i.files()).and_then(|f| f.get(0)) else {
                set_error.set(Some("Choose a file first.".into()));
                return;
            };
            if file.size() > f64::from(max_mb) * 1_048_576.0 {
                set_error.set(Some(format!("That file is too large (max {max_mb} MB).")));
                return;
            }
            let url = url.clone();
            set_busy.set(true);
            leptos::task::spawn_local(async move {
                let res = net::upload(&url, &file).await;
                set_busy.set(false);
                match res {
                    Ok(()) => {
                        if let Some(i) = input.get_untracked() {
                            i.set_value("");
                        }
                        set_chosen.set(false);
                        on_change.run(());
                    }
                    Err(e) => set_error.set(Some(e)),
                }
            });
        }
        #[cfg(not(feature = "hydrate"))]
        let _ = (&url, max_mb, input, on_change, set_busy, set_chosen);
    };

    let remove = move |_| {
        set_error.set(None);
        #[cfg(feature = "hydrate")]
        {
            let what = if kind == "ebook" { "Remove the PDF? Students won't be able to read this book online." } else { "Remove the cover?" };
            if !confirm(what) {
                return;
            }
            let url = url2.clone();
            set_busy.set(true);
            leptos::task::spawn_local(async move {
                let res = net::send(&url, "DELETE", None).await;
                set_busy.set(false);
                match res {
                    Ok(()) => on_change.run(()),
                    Err(e) => set_error.set(Some(e)),
                }
            });
        }
        #[cfg(not(feature = "hydrate"))]
        let _ = &url2;
    };

    let replace_label = if has_file { "Replace" } else { "Upload" };
    view! {
        <div class="flex flex-col gap-3">
            {move || error.get().map(|e| view! { <Alert variant=AlertVariant::Destructive>{e}</Alert> })}
            <input
                node_ref=input
                type="file"
                accept=accept
                aria-label=if kind == "ebook" { "PDF file" } else { "Cover image" }
                class="text-sm file:mr-3 file:rounded-md file:border file:border-border file:bg-background file:px-3 file:py-1.5 file:text-sm"
                on:change=move |_| set_chosen.set(true)
            />
            <div class="flex flex-wrap gap-2">
                <Button size=Size::Sm variant=Variant::Default on:click=upload
                    disabled=Signal::derive(move || busy.get() || !chosen.get())>
                    {move || if busy.get() { "Working…" } else { replace_label }}
                </Button>
                {has_file.then(|| view! {
                    <Button size=Size::Sm variant=Variant::Ghost on:click=remove disabled=busy>"Remove"</Button>
                })}
            </div>
        </div>
    }
}

#[cfg(feature = "hydrate")]
mod net {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    pub async fn upload(url: &str, file: &web_sys::File) -> Result<(), String> {
        let form = web_sys::FormData::new().map_err(|_| "Upload failed.".to_string())?;
        form.append_with_blob_and_filename("file", file, &file.name()).map_err(|_| "Upload failed.".to_string())?;
        send(url, "POST", Some(&form)).await
    }

    pub async fn send(url: &str, method: &str, body: Option<&web_sys::FormData>) -> Result<(), String> {
        let init = web_sys::RequestInit::new();
        init.set_method(method);
        init.set_credentials(web_sys::RequestCredentials::SameOrigin);
        if let Some(b) = body {
            init.set_body(b);
        }
        let window = web_sys::window().ok_or("no window")?;
        let resp = JsFuture::from(window.fetch_with_str_and_init(url, &init))
            .await
            .map_err(|_| "Network error. Check the connection and try again.".to_string())?;
        let resp: web_sys::Response = resp.unchecked_into();
        if resp.ok() {
            return Ok(());
        }
        let status = resp.status();
        let text = match resp.text() {
            Ok(p) => JsFuture::from(p).await.ok().and_then(|v| v.as_string()).unwrap_or_default(),
            Err(_) => String::new(),
        };
        Err(message(status, &text))
    }

    fn message(status: u16, body: &str) -> String {
        // API errors look like {"error":{"code":"…","message":"…"}}.
        let parsed = js_sys::JSON::parse(body).ok().and_then(|v| {
            let err = js_sys::Reflect::get(&v, &"error".into()).ok()?;
            js_sys::Reflect::get(&err, &"message".into()).ok()?.as_string()
        });
        match (status, parsed) {
            (_, Some(m)) if status != 500 => m,
            (401, _) => "Your session has ended. Log in again.".into(),
            (403, _) => "You don't have permission to do this.".into(),
            (408, _) => "The upload took too long. Try again on a faster connection.".into(),
            (413, _) => "That file is too large.".into(),
            _ => "Upload failed. Please try again.".into(),
        }
    }
}
