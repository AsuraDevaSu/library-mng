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

use domain::models::Role;
use leptos::prelude::*;
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::{
    StaticSegment,
    components::{A, ParentRoute, Route, Router, Routes},
    hooks::use_location,
};

use auth::AuthCtx;
use pages::{
    admin::{
        AdminLayout, audit::AuditPage, dashboard::DashboardPage, policies::PoliciesPage,
        settings::SettingsPage, staff::StaffPage,
    },
    books::BooksPage,
    desk::{AtDeskPage, DeskLayout, deposit::DepositPage, pickup::PickupPage},
    home::HomePage,
    login::LoginPage,
};
use ui::{Button, ButtonLink, Logo, Size, Variant};

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

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    provide_context(AuthCtx::new());

    view! {
        <Stylesheet id="leptos" href="/pkg/library.css"/>
        <Title text="Library"/>
        <Router>
            <SiteHeader/>
            <main class="mx-auto w-full max-w-7xl px-4 pb-4 sm:px-6 lg:px-10">
                <Routes fallback=|| view! { <p class="py-20 text-center text-xl">"Page not found."</p> }>
                    <Route path=StaticSegment("") view=HomePage/>
                    <Route path=StaticSegment("books") view=BooksPage/>
                    <Route path=StaticSegment("login") view=LoginPage/>
                    <ParentRoute path=StaticSegment("desk") view=DeskLayout>
                        <Route path=StaticSegment("") view=AtDeskPage/>
                        <Route path=StaticSegment("deposit") view=DepositPage/>
                        <Route path=StaticSegment("pickup") view=PickupPage/>
                    </ParentRoute>
                    <ParentRoute path=StaticSegment("admin") view=AdminLayout>
                        <Route path=StaticSegment("") view=DashboardPage/>
                        <Route path=StaticSegment("staff") view=StaffPage/>
                        <Route path=StaticSegment("policies") view=PoliciesPage/>
                        <Route path=StaticSegment("settings") view=SettingsPage/>
                        <Route path=StaticSegment("audit") view=AuditPage/>
                    </ParentRoute>
                </Routes>
            </main>
            <SiteFooter/>
        </Router>
    }
}

const NAV_LINK: &str = "whitespace-nowrap rounded-md px-1 py-0.5 text-[17px] hover:bg-accent aria-[current=page]:bg-accent";

#[component]
fn SiteHeader() -> impl IntoView {
    view! {
        <header class="mx-auto flex w-full max-w-7xl flex-wrap items-center gap-x-8 gap-y-3 px-4 py-6 sm:px-6 lg:px-10">
            <A href="/" attr:class="flex items-center gap-2 text-2xl font-bold tracking-tight">
                <Logo/>
                "Library"
            </A>
            <nav class="order-3 flex w-full gap-6 overflow-x-auto md:order-none md:ml-auto md:w-auto md:gap-8">
                <A href="/books" attr:class=NAV_LINK>"Catalog"</A>
                <a href="/#services" class=NAV_LINK>"Services"</a>
                <a href="/#how-it-works" class=NAV_LINK>"How it works"</a>
                <a href="/#visit" class=NAV_LINK>"Visit"</a>
            </nav>
            <div class="ml-auto md:ml-0">
                <UserMenu/>
            </div>
        </header>
    }
}

/// "Staff login" button, or the signed-in user with Admin / Log out.
#[component]
fn UserMenu() -> impl IntoView {
    let auth = expect_context::<AuthCtx>();
    view! {
        <Transition fallback=|| ()>
            {move || Suspend::new(async move {
                match auth.user.await {
                    Ok(Some(user)) => {
                        let is_admin = user.role == Role::Admin;
                        view! {
                            <div class="flex items-center gap-3">
                                <ButtonLink href="/desk" variant=Variant::Outline size=Size::Sm>"Bag desk"</ButtonLink>
                                {is_admin.then(|| view! {
                                    <ButtonLink href="/admin" variant=Variant::Brand size=Size::Sm>"Admin"</ButtonLink>
                                })}
                                <span class="hidden text-sm text-muted-foreground lg:inline">
                                    {user.full_name} " · " {user.role.label()}
                                </span>
                                <ActionForm action=auth.logout>
                                    <Button kind="submit" variant=Variant::Outline size=Size::Sm>"Log out"</Button>
                                </ActionForm>
                            </div>
                        }
                            .into_any()
                    }
                    _ => view! {
                        <ButtonLink href="/login" variant=Variant::Outline>"Staff login"</ButtonLink>
                    }
                        .into_any(),
                }
            })}
        </Transition>
    }
}

#[component]
fn SiteFooter() -> impl IntoView {
    let location = use_location();
    let info = Resource::new(|| (), |_| server_fns::get_site_info());
    view! {
        <Show when=move || {
            let p = location.pathname.get();
            !p.starts_with("/admin") && !p.starts_with("/desk")
        }>
            <footer class="mx-auto w-full max-w-7xl px-4 sm:px-6 lg:px-10">
                <div class="rounded-t-card bg-primary px-8 pt-12 pb-8 text-primary-foreground md:px-14">
                    <div class="flex flex-col gap-8 md:flex-row md:items-center md:justify-between">
                        <A href="/" attr:class="flex items-center gap-2 text-2xl font-bold">
                            <Logo light=true/>
                            "Library"
                        </A>
                        <nav class="flex flex-wrap gap-6 underline-offset-4 [&_a:hover]:underline">
                            <A href="/books">"Catalog"</A>
                            <a href="/#services">"Services"</a>
                            <a href="/#how-it-works">"How it works"</a>
                            <a href="/#visit">"Visit"</a>
                            <A href="/login">"Staff login"</A>
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
        </Show>
    }
}
