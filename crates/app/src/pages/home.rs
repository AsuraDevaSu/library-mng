//! Public landing page, laid out after the Positivus design:
//! hero → numbers strip → services grid → CTA → process accordion → visit block.

use leptos::prelude::*;
use leptos_router::components::A;

use crate::{
    server_fns::get_site_info,
    ui::{ButtonLink, Card, Highlight, LearnMore, SectionHeading, Size, Tone, Variant, cn},
};

#[component]
pub fn HomePage() -> impl IntoView {
    view! {
        <Hero/>
        <Numbers/>
        <Services/>
        <CallToAction/>
        <HowItWorks/>
        <Visit/>
    }
}

#[component]
fn Hero() -> impl IntoView {
    view! {
        <section class="grid items-center gap-10 py-10 md:grid-cols-2 md:py-16">
            <div class="flex flex-col gap-8">
                <h1 class="text-5xl leading-[1.05] font-medium tracking-tight md:text-6xl">
                    "Read, borrow and study, all in one place"
                </h1>
                <p class="max-w-lg text-lg md:text-xl">
                    "Search thousands of books, check what's on the shelf right now, and borrow at the desk with your student card."
                </p>
                <div class="flex flex-wrap gap-4">
                    <ButtonLink href="/books" size=Size::Lg>"Browse the catalog"</ButtonLink>
                    <a href="#how-it-works" class=crate::ui::button_class(Variant::Outline, Size::Lg)>"How it works"</a>
                </div>
            </div>
            <HeroArt/>
        </section>
    }
}

/// Line-art book stack with lime accents (in the spirit of Positivus' 3D line illustrations).
#[component]
fn HeroArt() -> impl IntoView {
    view! {
        <svg viewBox="0 0 520 440" class="mx-auto w-full max-w-lg" role="img" aria-label="Stack of books with an open book on top">
            <g fill="none" stroke="#191A23" stroke-width="3" stroke-linejoin="round" stroke-linecap="round">
                // orbit ring
                <ellipse cx="260" cy="250" rx="235" ry="70" transform="rotate(-12 260 250)" stroke-width="2.5"/>
                // bottom book
                <path d="M110 330h300l-20 46H90z" fill="#F3F3F3"/>
                <path d="M90 376h300v18H90z" fill="#191A23"/>
                // middle book (lime)
                <path d="M140 270h270l-14 50H126z" fill="#B9FF66"/>
                <path d="M126 320h270v14H126z" fill="#191A23"/>
                <path d="M170 286h170M162 302h170"/>
                // top book
                <path d="M120 222h260l-10 40H110z" fill="#fff"/>
                <path d="M110 262h260v12H110z" fill="#191A23"/>
                // open book
                <path d="M260 196c-40-22-88-26-128-12v-74c40-14 88-10 128 12z" fill="#fff"/>
                <path d="M260 196c40-22 88-26 128-12v-74c-40-14-88-10-128 12z" fill="#B9FF66"/>
                <path d="M260 122v74"/>
                <path d="M152 128c28-8 60-6 88 6M152 148c28-8 60-6 88 6M280 134c28-12 60-14 88-6M280 154c28-12 60-14 88-6"/>
            </g>
            // sparkles
            <path d="M70 120l8 22 22 8-22 8-8 22-8-22-22-8 22-8z" fill="#191A23"/>
            <path d="M450 70l6 16 16 6-16 6-6 16-6-16-16-6 16-6z" fill="#B9FF66" stroke="#191A23" stroke-width="2.5"/>
            <path d="M440 300l5 13 13 5-13 5-5 13-5-13-13-5 13-5z" fill="#191A23"/>
            <circle cx="420" cy="170" r="22" fill="#191A23"/>
            <path d="M410 170h20M420 160v20" stroke="#B9FF66" stroke-width="4" stroke-linecap="round"/>
            <circle cx="96" cy="250" r="16" fill="#B9FF66" stroke="#191A23" stroke-width="2.5"/>
        </svg>
    }
}

#[component]
fn Numbers() -> impl IntoView {
    let info = Resource::new(|| (), |_| get_site_info());
    let stat = |value: i64, label: &'static str| {
        view! {
            <div class="flex flex-col">
                <span class="text-4xl font-medium tabular-nums md:text-5xl">{value.to_string()}</span>
                <span class="text-muted-foreground">{label}</span>
            </div>
        }
    };
    view! {
        <section class="mb-20 grid grid-cols-2 gap-8 border-y border-hairline py-8 md:grid-cols-4">
            <Suspense fallback=|| view! { <p class="col-span-full text-muted-foreground">"Loading…"</p> }>
                {move || Suspend::new(async move {
                    info.await.map(|i| view! {
                        {stat(i.titles, "Book titles")}
                        {stat(i.copies, "Copies on our shelves")}
                        {stat(i.available_copies, "Available right now")}
                        {stat(i.members, "Active members")}
                    })
                })}
            </Suspense>
        </section>
    }
}

struct Service {
    title: [&'static str; 2],
    href: &'static str,
    tone: Tone,
    icon: fn() -> AnyView,
}

#[component]
fn Services() -> impl IntoView {
    let services = [
        Service { title: ["Search the", "catalog"], href: "/books", tone: Tone::Muted, icon: icon_search },
        Service { title: ["Borrow &", "renew books"], href: "#how-it-works", tone: Tone::Brand, icon: icon_books },
        Service { title: ["Quiet", "reading room"], href: "#visit", tone: Tone::Dark, icon: icon_lamp },
        Service { title: ["Bag deposit", "desk"], href: "#visit", tone: Tone::Brand, icon: icon_bag },
        Service { title: ["Study", "support"], href: "#visit", tone: Tone::Dark, icon: icon_chat },
        Service { title: ["Live shelf", "availability"], href: "/books", tone: Tone::Muted, icon: icon_shelf },
    ];

    view! {
        <section class="mb-24">
            <SectionHeading
                id="services"
                title="Services"
                description="Everything the library offers, from finding a book to keeping your bag safe while you study."
            />
            <div class="grid gap-8 md:grid-cols-2">
                {services
                    .into_iter()
                    .map(|s| {
                        let on_dark = s.tone == Tone::Dark;
                        let white_title = s.tone != Tone::Muted;
                        view! {
                            <Card tone=s.tone large=true class="p-8 md:p-12">
                                <a href=s.href class="flex min-h-56 items-stretch justify-between gap-6 outline-none focus-visible:ring-4 focus-visible:ring-ring/70 rounded-3xl">
                                    <div class="flex flex-col justify-between gap-10">
                                        <h3 class="flex flex-col items-start gap-1 text-2xl font-medium md:text-3xl">
                                            <Highlight white=white_title>{s.title[0]}</Highlight>
                                            <Highlight white=white_title>{s.title[1]}</Highlight>
                                        </h3>
                                        <LearnMore on_dark=on_dark label="Learn more"/>
                                    </div>
                                    <div class=cn(&["hidden shrink-0 self-center sm:block", if on_dark { "text-white" } else { "text-foreground" }])>
                                        {(s.icon)()}
                                    </div>
                                </a>
                            </Card>
                        }
                    })
                    .collect_view()}
            </div>
        </section>
    }
}

// Large line icons for the service cards (stroke = currentColor, lime fills).
fn svg(children: AnyView) -> AnyView {
    view! {
        <svg viewBox="0 0 120 120" class="size-36 md:size-40" fill="none" stroke="currentColor" stroke-width="3"
            stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">{children}</svg>
    }
    .into_any()
}
fn icon_search() -> AnyView {
    svg(view! { <circle cx="52" cy="52" r="30" fill="#B9FF66"/><path d="M74 74l26 26"/><path d="M40 46h24M40 58h16"/> }.into_any())
}
fn icon_books() -> AnyView {
    svg(view! { <rect x="18" y="24" width="20" height="74" rx="3" fill="#fff"/><rect x="42" y="16" width="22" height="82" rx="3" fill="#191A23"/><path d="M72 30l18-5 20 70-18 5z" fill="#fff"/><path d="M12 98h100"/> }.into_any())
}
fn icon_lamp() -> AnyView {
    svg(view! { <path d="M40 18h40l14 34H26z" fill="#B9FF66" stroke="#191A23"/><path d="M60 52v40M36 100h48"/><path d="M28 62l-10 8M92 62l10 8M60 64v6" stroke="#B9FF66"/> }.into_any())
}
fn icon_bag() -> AnyView {
    svg(view! { <path d="M30 44h60l6 58H24z" fill="#fff"/><path d="M44 44V34a16 16 0 0 1 32 0v10"/><rect x="48" y="62" width="24" height="16" rx="3" fill="#191A23"/><text x="60" y="74" font-size="10" text-anchor="middle" fill="#B9FF66" stroke="none" font-family="Space Grotesk">"B-12"</text> }.into_any())
}
fn icon_chat() -> AnyView {
    svg(view! { <path d="M14 22h64v40H40l-14 14V62H14z" fill="#B9FF66" stroke="#191A23"/><path d="M84 44h22v38H96v12L84 82H58V70"/><path d="M28 38h36M28 48h24" stroke="#191A23"/> }.into_any())
}
fn icon_shelf() -> AnyView {
    svg(view! { <path d="M12 98h96M12 58h96"/><rect x="20" y="20" width="12" height="38" fill="#B9FF66"/><rect x="36" y="26" width="12" height="32" fill="#191A23"/><rect x="52" y="20" width="12" height="38"/><rect x="24" y="66" width="12" height="32"/><rect x="40" y="62" width="12" height="36" fill="#B9FF66"/><circle cx="88" cy="38" r="12" fill="#B9FF66"/><path d="M83 38l4 4 7-8"/> }.into_any())
}

#[component]
fn CallToAction() -> impl IntoView {
    view! {
        <section class="mb-24">
            <Card tone=Tone::Muted large=true class="relative overflow-hidden p-10 md:p-14">
                <div class="flex flex-col gap-6 md:max-w-lg">
                    <h3 class="text-3xl font-medium">"Looking for a specific book?"</h3>
                    <p class="text-lg">
                        "Search by title, author or ISBN and see how many copies are on the shelf before you come in."
                    </p>
                    <div>
                        <ButtonLink href="/books" size=Size::Lg>"Search the catalog"</ButtonLink>
                    </div>
                </div>
                <svg viewBox="0 0 200 200" class="absolute -right-6 -bottom-8 hidden w-72 md:block" aria-hidden="true">
                    <circle cx="100" cy="100" r="70" fill="#191A23"/>
                    <circle cx="78" cy="88" r="10" fill="#fff"/><circle cx="122" cy="88" r="10" fill="#fff"/>
                    <path d="M76 122c14 12 34 12 48 0" stroke="#fff" stroke-width="6" fill="none" stroke-linecap="round"/>
                    <path d="M160 30l6 16 16 6-16 6-6 16-6-16-16-6 16-6z" fill="#B9FF66" stroke="#191A23" stroke-width="2"/>
                </svg>
            </Card>
        </section>
    }
}

#[component]
fn HowItWorks() -> impl IntoView {
    let steps = [
        ("Register at the desk", "Show your student card or ID at the front desk. Staff create your library account in a minute, and your card becomes your library card."),
        ("Find your book", "Search the catalog on this site or at the library computers. The availability badge tells you how many copies are on the shelf right now."),
        ("Borrow at the counter", "Bring the book and your card to the counter. The due date depends on your membership type and is printed on your receipt."),
        ("Return or renew", "Return books to the counter before the due date. You can renew a loan if nobody else is waiting for it. Late returns add a small daily fine."),
        ("Leave your bag at the deposit desk", "Bags stay at the deposit desk while you read. Staff take a photo of you and your bag so it's returned to the right person."),
    ];
    view! {
        <section class="mb-24">
            <SectionHeading
                id="how-it-works"
                title="How it works"
                description="From your first visit to returning your books: five simple steps."
            />
            <div class="flex flex-col gap-6">
                {steps
                    .into_iter()
                    .enumerate()
                    .map(|(i, (title, body))| view! {
                        <details
                            open=i == 0
                            class="group rounded-card border border-border bg-muted shadow-brut open:bg-accent"
                        >
                            <summary class="flex cursor-pointer items-center gap-6 px-8 py-8 md:px-14">
                                <span class="text-4xl font-medium md:text-6xl">{format!("{:02}", i + 1)}</span>
                                <span class="flex-1 text-xl font-medium md:text-3xl">{title}</span>
                                <span class="grid size-12 shrink-0 place-items-center rounded-full border border-border bg-muted text-3xl leading-none">
                                    <span class="group-open:hidden">"+"</span>
                                    <span class="hidden group-open:inline">"−"</span>
                                </span>
                            </summary>
                            <div class="mx-8 border-t border-border pt-6 pb-10 text-lg md:mx-14">{body}</div>
                        </details>
                    })
                    .collect_view()}
            </div>
        </section>
    }
}

#[component]
fn Visit() -> impl IntoView {
    let info = Resource::new(|| (), |_| get_site_info());
    view! {
        <section class="mb-24">
            <SectionHeading
                id="visit"
                title="Visit us"
                description="Opening hours and what to bring. Staff at the front desk are happy to help."
            />
            <Card tone=Tone::Dark large=true class="grid gap-10 p-10 md:grid-cols-3 md:gap-0 md:p-14">
                <div class="md:border-r md:border-white/40 md:pr-10">
                    <h3 class="mb-3 text-xl font-medium text-accent">"Opening hours"</h3>
                    <Suspense fallback=|| view! { <p>"…"</p> }>
                        {move || Suspend::new(async move {
                            info.await.map(|i| view! {
                                <p class="text-lg">{i.opening_hours}</p>
                                <p class="mt-2 text-white/70">{i.library_name}</p>
                            })
                        })}
                    </Suspense>
                </div>
                <div class="md:border-r md:border-white/40 md:px-10">
                    <h3 class="mb-3 text-xl font-medium text-accent">"Borrowing"</h3>
                    <p class="text-white/90">
                        "Bring your student card. How many books you can borrow and for how long depends on your membership type."
                    </p>
                    <A href="/books" attr:class="mt-4 inline-flex items-center gap-2 text-accent hover:underline">"Check availability →"</A>
                </div>
                <div class="md:pl-10">
                    <h3 class="mb-3 text-xl font-medium text-accent">"Bag deposit"</h3>
                    <p class="text-white/90">
                        "Leave your bag at the deposit desk. Photos taken at drop-off are only used to return your bag and are deleted afterwards."
                    </p>
                </div>
            </Card>
        </section>
    }
}
