pub mod audit;
pub mod dashboard;
pub mod policies;
pub mod settings;
pub mod staff;

use domain::models::Role;
use leptos::prelude::*;
use leptos_router::components::{A, Outlet, Redirect};

use crate::{
    auth::AuthCtx,
    ui::{Alert, AlertVariant, Highlight},
};

const NAV: &str = "whitespace-nowrap rounded-lg px-4 py-2.5 transition-colors hover:bg-white/10 \
                   aria-[current=page]:bg-accent aria-[current=page]:text-accent-foreground aria-[current=page]:font-medium";

/// Sidebar + page. Non-admins are sent to /login; the server functions behind
/// every admin page enforce `Role::Admin` independently of this.
#[component]
pub fn AdminLayout() -> impl IntoView {
    let auth = expect_context::<AuthCtx>();
    view! {
        <Transition fallback=|| ()>
            {move || Suspend::new(async move {
                match auth.user.await {
                    Ok(Some(u)) if u.role == Role::Admin => ().into_any(),
                    Ok(Some(_)) => view! {
                        <Alert variant=AlertVariant::Destructive>"This area is for admins only."</Alert>
                    }
                    .into_any(),
                    _ => view! { <Redirect path="/login"/> }.into_any(),
                }
            })}
        </Transition>
        <div class="grid grid-cols-[minmax(0,1fr)] items-start gap-8 py-4 lg:grid-cols-[230px_minmax(0,1fr)]">
            <aside class="rounded-3xl border border-border bg-primary p-3 text-primary-foreground shadow-brut lg:sticky lg:top-6">
                <p class="hidden px-4 pt-2 pb-3 text-xs tracking-widest text-white/60 uppercase lg:block">"Admin"</p>
                <nav class="flex gap-1 overflow-x-auto lg:flex-col">
                    <A href="/admin" exact=true attr:class=NAV>"Dashboard"</A>
                    <A href="/admin/staff" attr:class=NAV>"Staff"</A>
                    <A href="/admin/policies" attr:class=NAV>"Borrowing rules"</A>
                    <A href="/admin/settings" attr:class=NAV>"Settings"</A>
                    <A href="/admin/audit" attr:class=NAV>"Audit log"</A>
                </nav>
            </aside>
            <section class="min-w-0">
                <Outlet/>
            </section>
        </div>
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
