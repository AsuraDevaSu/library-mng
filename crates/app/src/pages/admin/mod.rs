pub mod audit;
pub mod books;
pub mod dashboard;
pub mod policies;
pub mod settings;
pub mod staff;

use domain::models::Role;
use leptos::prelude::*;
use leptos_router::components::{A, Outlet, Redirect};

use crate::{
    auth::AuthCtx,
    ui::{Button, Highlight, Logo, Size, Variant},
};

const NAV: &str = "whitespace-nowrap rounded-lg px-4 py-2.5 transition-colors hover:bg-white/10 \
                   aria-[current=page]:bg-accent aria-[current=page]:text-accent-foreground aria-[current=page]:font-medium";

/// Top bar of the staff area (admin + bag desk). Separate from the public
/// site: students never see staff links, and staff log in at `/admin/login`.
#[component]
pub fn StaffBar() -> impl IntoView {
    let auth = expect_context::<AuthCtx>();
    let link = "whitespace-nowrap rounded-md px-3 py-1.5 hover:bg-white/10 aria-[current=page]:bg-white/15";
    view! {
        <header class="print:hidden bg-primary text-primary-foreground">
            <div class="mx-auto flex w-full max-w-7xl flex-wrap items-center gap-x-6 gap-y-2 px-4 py-3 sm:px-6 lg:px-10">
                <a href="/admin" class="flex items-center gap-2 text-lg font-bold">
                    <Logo light=true/>
                    "Library"
                    <span class="rounded bg-accent px-1.5 py-0.5 text-xs font-medium text-accent-foreground">"Staff"</span>
                </a>
                <Transition fallback=|| ()>
                    {move || Suspend::new(async move {
                        auth.user.await.ok().flatten().map(|user| {
                            let librarian = user.role.satisfies(Role::Librarian);
                            view! {
                                <nav class="flex gap-1 text-sm">
                                    {librarian.then(|| view! { <A href="/admin" attr:class=link>"Admin"</A> })}
                                    <A href="/desk" attr:class=link>"Bag desk"</A>
                                </nav>
                                <div class="ml-auto flex items-center gap-3 text-sm">
                                    <a href="/" target="_blank" rel="noopener" class="hidden text-white/70 hover:text-white hover:underline sm:inline">
                                        "Public site ↗"
                                    </a>
                                    <span class="hidden text-white/70 md:inline">{user.full_name} " · " {user.role.label()}</span>
                                    <ActionForm action=auth.logout>
                                        <Button kind="submit" variant=Variant::Outline size=Size::Sm class="text-foreground">"Log out"</Button>
                                    </ActionForm>
                                </div>
                            }
                        })
                    })}
                </Transition>
            </div>
        </header>
    }
}

/// Signed-out staff go to the staff login; staff below `role` go to `fallback`.
#[component]
pub fn RequireRole(role: Role, fallback: &'static str) -> impl IntoView {
    let auth = expect_context::<AuthCtx>();
    view! {
        <Transition fallback=|| ()>
            {move || Suspend::new(async move {
                match auth.user.await {
                    Ok(Some(u)) if u.role.satisfies(role) => ().into_any(),
                    Ok(Some(_)) => view! { <Redirect path=fallback/> }.into_any(),
                    _ => view! { <Redirect path="/admin/login"/> }.into_any(),
                }
            })}
        </Transition>
    }
}

/// Staff bar + sidebar + page, for librarians and admins. Admin-only pages
/// are hidden from librarians here, and every server function behind them
/// checks the role again on the server.
#[component]
pub fn AdminLayout() -> impl IntoView {
    let auth = expect_context::<AuthCtx>();
    view! {
        <StaffBar/>
        <RequireRole role=Role::Librarian fallback="/desk"/>
        <main class="mx-auto w-full max-w-7xl px-4 pb-10 sm:px-6 lg:px-10">
            <div class="grid grid-cols-[minmax(0,1fr)] items-start gap-8 py-6 lg:grid-cols-[230px_minmax(0,1fr)]">
                <aside class="rounded-3xl border border-border bg-primary p-3 text-primary-foreground shadow-brut lg:sticky lg:top-6">
                    <p class="hidden px-4 pt-2 pb-3 text-xs tracking-widest text-white/60 uppercase lg:block">"Admin"</p>
                    <Transition fallback=|| ()>
                        {move || Suspend::new(async move {
                            let admin = matches!(auth.user.await, Ok(Some(ref u)) if u.role == Role::Admin);
                            view! {
                                <nav class="flex gap-1 overflow-x-auto lg:flex-col">
                                    {admin.then(|| view! { <A href="/admin" exact=true attr:class=NAV>"Dashboard"</A> })}
                                    <A href="/admin/books" attr:class=NAV>"Books"</A>
                                    {admin.then(|| view! {
                                        <A href="/admin/staff" attr:class=NAV>"Staff"</A>
                                        <A href="/admin/policies" attr:class=NAV>"Borrowing rules"</A>
                                        <A href="/admin/settings" attr:class=NAV>"Settings"</A>
                                        <A href="/admin/audit" attr:class=NAV>"Audit log"</A>
                                    })}
                                </nav>
                            }
                        })}
                    </Transition>
                </aside>
                <section class="min-w-0">
                    <Outlet/>
                </section>
            </div>
        </main>
    }
}

/// Page title with the Positivus highlight and an optional subtitle.
#[component]
pub(crate) fn PageTitle(#[prop(into)] title: String, #[prop(optional, into)] subtitle: String) -> impl IntoView {
    view! {
        <div class="mb-8">
            <h1 class="text-3xl font-medium md:text-4xl"><Highlight>{title}</Highlight></h1>
            {(!subtitle.is_empty()).then(|| view! { <p class="mt-3 text-muted-foreground">{subtitle}</p> })}
        </div>
    }
}

/// Formats a UTC timestamp for tables.
pub(crate) fn fmt_time<T: std::fmt::Display>(t: T) -> String {
    // DateTime<Utc> displays as "2026-11-17 07:42:48.990327 UTC"; trim seconds.
    let s = t.to_string();
    s.get(..16).map(|p| format!("{p} UTC")).unwrap_or(s)
}
