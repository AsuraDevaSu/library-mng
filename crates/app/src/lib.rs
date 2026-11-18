//! Leptos UI. Compiled twice: with `ssr` into the server, with `hydrate` into WASM.

/// Renders a server action's last result as a shadcn-style Alert.
#[macro_export]
macro_rules! outcome {
    ($action:expr, $ok:expr) => {{
        let action = $action;
        move || {
            action.value().get().map(|r| match r {
                Ok(_) => leptos::prelude::view! {
                    <$crate::ui::Alert variant=$crate::ui::AlertVariant::Success>{$ok}</$crate::ui::Alert>
                }
                .into_any(),
                Err(e) => leptos::prelude::view! {
                    <$crate::ui::Alert variant=$crate::ui::AlertVariant::Destructive>
                        {$crate::auth::err_msg(&e)}
                    </$crate::ui::Alert>
                }
                .into_any(),
            })
        }
    }};
}

pub mod admin_fns;
pub mod auth;
pub mod camera;
pub mod desk_fns;
pub mod pages;
pub mod server_fns;
pub mod ui;

use leptos::prelude::*;
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::{
    ParamSegment, StaticSegment,
    components::{A, Outlet, ParentRoute, Redirect, Route, Router, Routes},
};

use auth::AuthCtx;
use pages::{
    admin::{
        AdminLayout,
        audit::AuditPage,
        books::{AdminBooksPage, EditBookPage, NewBookPage},
        dashboard::DashboardPage,
        policies::PoliciesPage,
        settings::SettingsPage,
        staff::StaffPage,
    },
    books::{BookPage, BooksPage, ReaderPage},
    desk::{AtDeskPage, DeskLayout, deposit::DepositPage, pickup::PickupPage},
    home::HomePage,
    login::LoginPage,
};
use ui::{ButtonLink, Logo, Variant};

/// HTML document shell rendered on the server.
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <link rel="preconnect" href="https://fonts.googleapis.com"/>
                <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin=""/>
                <link
                    rel="stylesheet"
                    href="https://fonts.googleapis.com/css2?family=Space+Grotesk:wght@400;500;700&display=swap"
                />
                <AutoReload options=options.clone()/>
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

/// Two separate sites in one app:
/// - the public library (`/`, `/books…`) for students and teachers — no login;
/// - the staff area (`/admin…`, `/desk…`) with its own login at `/admin/login`.
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    provide_context(AuthCtx::new());

    view! {
        <Stylesheet id="leptos" href="/pkg/library.css"/>
        <Title text="Library"/>
        <Router>
            <Routes fallback=|| view! {
                <PublicHeader/>
                <p class="py-20 text-center text-xl">"Page not found."</p>
            }>
                // ---- public library ----
                <ParentRoute path=StaticSegment("") view=PublicLayout>
                    <Route path=StaticSegment("") view=HomePage/>
                    <Route path=StaticSegment("books") view=BooksPage/>
                    <Route path=(StaticSegment("books"), ParamSegment("id")) view=BookPage/>
                </ParentRoute>
                <Route path=(StaticSegment("books"), ParamSegment("id"), StaticSegment("read")) view=ReaderPage/>

                // ---- staff area ----
                <Route path=(StaticSegment("admin"), StaticSegment("login")) view=LoginPage/>
                <Route path=StaticSegment("login") view=|| view! { <Redirect path="/admin/login"/> }/>
                <ParentRoute path=StaticSegment("admin") view=AdminLayout>
                    <Route path=StaticSegment("") view=DashboardPage/>
                    <Route path=StaticSegment("books") view=AdminBooksPage/>
                    <Route path=(StaticSegment("books"), StaticSegment("new")) view=NewBookPage/>
                    <Route path=(StaticSegment("books"), ParamSegment("id")) view=EditBookPage/>
                    <Route path=StaticSegment("staff") view=StaffPage/>
                    <Route path=StaticSegment("policies") view=PoliciesPage/>
                    <Route path=StaticSegment("settings") view=SettingsPage/>
                    <Route path=StaticSegment("audit") view=AuditPage/>
                </ParentRoute>
                <ParentRoute path=StaticSegment("desk") view=DeskLayout>
                    <Route path=StaticSegment("") view=AtDeskPage/>
                    <Route path=StaticSegment("deposit") view=DepositPage/>
                    <Route path=StaticSegment("pickup") view=PickupPage/>
                </ParentRoute>
            </Routes>
        </Router>
    }
}

#[component]
fn PublicLayout() -> impl IntoView {
    view! {
        <PublicHeader/>
        <main class="mx-auto w-full max-w-7xl px-4 pb-4 sm:px-6 lg:px-10">
            <Outlet/>
        </main>
        <SiteFooter/>
    }
}

const NAV_LINK: &str = "whitespace-nowrap rounded-md px-1 py-0.5 text-[17px] hover:bg-accent aria-[current=page]:bg-accent";

/// Public header: catalog and library info only — no staff links.
#[component]
fn PublicHeader() -> impl IntoView {
    view! {
        <header class="print:hidden mx-auto flex w-full max-w-7xl flex-wrap items-center gap-x-8 gap-y-3 px-4 py-6 sm:px-6 lg:px-10">
            <A href="/" attr:class="flex items-center gap-2 text-2xl font-bold tracking-tight">
                <Logo/>
                "Library"
            </A>
            <nav class="order-3 flex w-full gap-6 overflow-x-auto md:order-none md:ml-auto md:w-auto md:gap-8">
                <A href="/books" attr:class=NAV_LINK>"Catalog"</A>
                <a href="/books?online=1" class=NAV_LINK>"Read online"</a>
                <a href="/#services" class=NAV_LINK>"Services"</a>
                <a href="/#visit" class=NAV_LINK>"Visit"</a>
            </nav>
            <div class="ml-auto md:ml-0">
                <ButtonLink href="/books" variant=Variant::Outline>"Find a book"</ButtonLink>
            </div>
        </header>
    }
}

#[component]
fn SiteFooter() -> impl IntoView {
    let info = Resource::new(|| (), |_| server_fns::get_site_info());
    view! {
        <footer class="print:hidden mx-auto w-full max-w-7xl px-4 sm:px-6 lg:px-10">
            <div class="rounded-t-card bg-primary px-8 pt-12 pb-8 text-primary-foreground md:px-14">
                <div class="flex flex-col gap-8 md:flex-row md:items-center md:justify-between">
                    <A href="/" attr:class="flex items-center gap-2 text-2xl font-bold">
                        <Logo light=true/>
                        "Library"
                    </A>
                    <nav class="flex flex-wrap gap-6 underline-offset-4 [&_a:hover]:underline">
                        <A href="/books">"Catalog"</A>
                        <a href="/books?online=1">"Read online"</a>
                        <a href="/#services">"Services"</a>
                        <a href="/#how-it-works">"How it works"</a>
                        <a href="/#visit">"Visit"</a>
                    </nav>
                </div>
                <div class="mt-10 grid gap-8 md:grid-cols-2">
                    <div class="space-y-3">
                        <ui::Highlight class="text-lg font-medium">"Opening hours"</ui::Highlight>
                        <Suspense fallback=|| ()>
                            {move || Suspend::new(async move {
                                info.await.map(|i| view! { <p class="text-white/80">{i.opening_hours}</p> })
                            })}
                        </Suspense>
                    </div>
                    <div class="rounded-2xl bg-white/10 p-6 text-white/80">
                        "Bring your student card to borrow books. Bags are kept safely at the deposit desk while you read."
                    </div>
                </div>
                <hr class="my-8 border-white/30"/>
                <p class="text-sm text-white/70">"© Library management system · Built with Rust & Leptos"</p>
            </div>
        </footer>
    }
}
