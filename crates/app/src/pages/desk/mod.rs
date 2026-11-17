//! Bag deposit desk (ដាក់ផ្ញើកាតាប): at-desk list, drop-off and pickup.

pub mod deposit;
pub mod pickup;

use domain::models::{BagDeposit, Role};
use leptos::prelude::*;
use leptos_router::{
    components::{A, Outlet, Redirect},
    hooks::use_location,
};

use crate::{
    auth::{AuthCtx, err_msg},
    desk_fns::{HandOverBag, desk_bags},
    pages::admin::PageTitle,
    ui::{Alert, AlertVariant, Badge, BadgeVariant, Button, Card, INPUT, Size, Tone, Variant, cn},
};

const TAB: &str = "whitespace-nowrap rounded-lg px-4 py-2.5 font-medium transition-colors hover:bg-white/10 \
                   aria-[current=page]:bg-accent aria-[current=page]:text-accent-foreground";

#[component]
pub fn DeskLayout() -> impl IntoView {
    let auth = expect_context::<AuthCtx>();
    let location = use_location();
    // Refresh the tag counter whenever staff switch tabs.
    let summary = Resource::new(move || location.pathname.get(), |_| desk_bags(String::new()));

    view! {
        <Transition fallback=|| ()>
            {move || Suspend::new(async move {
                match auth.user.await {
                    Ok(Some(u)) if u.role.satisfies(Role::DeskStaff) => ().into_any(),
                    _ => view! { <Redirect path="/login"/> }.into_any(),
                }
            })}
        </Transition>
        <div class="py-4">
            <div class="mb-8 flex flex-col gap-4 rounded-3xl border border-border bg-primary p-3 text-primary-foreground shadow-brut md:flex-row md:items-center">
                <nav class="flex gap-1 overflow-x-auto">
                    <A href="/desk" exact=true attr:class=TAB>"At the desk"</A>
                    <A href="/desk/deposit" attr:class=TAB>"Drop-off"</A>
                    <A href="/desk/pickup" attr:class=TAB>"Pickup"</A>
                </nav>
                <div class="px-4 text-sm text-white/80 md:ml-auto">
                    <Transition fallback=|| ()>
                        {move || Suspend::new(async move {
                            summary.await.ok().map(|(s, _)| view! {
                                <span class="font-medium text-accent tabular-nums">{s.in_use} " / " {s.capacity}</span>
                                " tags in use"
                                {(!s.face_search).then(|| view! { <span class="ml-3 text-white/60">"· face search off"</span> })}
                            })
                        })}
                    </Transition>
                </div>
            </div>
            <Outlet/>
        </div>
    }
}

/// Colour dot + name.
#[component]
pub fn ColorSwatch(#[prop(into)] name: String, #[prop(into)] hex: String) -> impl IntoView {
    view! {
        <span class="inline-flex items-center gap-2">
            <span class="size-5 shrink-0 rounded-full border border-border" style=format!("background:{hex}")></span>
            {name}
        </span>
    }
}

/// Photos + tag + colour of one deposit, with a "Hand over" button.
#[component]
pub fn BagCard(
    deposit: BagDeposit,
    hand_over: ServerAction<HandOverBag>,
    /// How staff identified the student: "face", "tag" or "name".
    method: &'static str,
    #[prop(optional)] score: Option<f32>,
    #[prop(optional)] confident: bool,
    #[prop(optional)] highlight: bool,
) -> impl IntoView {
    let d = deposit;
    let photo = |kind: &str| format!("/api/v1/bags/{}/photo/{kind}", d.id);
    let face_src = photo("face");
    let bag_src = photo("bag");
    let id = d.id;
    view! {
        <Card tone={if highlight { Tone::Brand } else { Tone::Plain }} class="@container p-5">
          <div class="flex flex-col gap-5 @lg:flex-row">
            <div class="grid shrink-0 grid-cols-2 gap-3 @lg:w-64">
                <figure class="flex flex-col gap-1">
                    {if d.has_face_photo {
                        view! { <img src=face_src alt=format!("Face photo of {}", d.student_name) class="aspect-square w-full rounded-2xl border border-border bg-muted object-cover"/> }.into_any()
                    } else {
                        view! { <div class="grid aspect-square w-full place-items-center rounded-2xl border border-dashed border-border bg-muted p-2 text-center text-xs text-muted-foreground">"No face photo (opted out)"</div> }.into_any()
                    }}
                    <figcaption class="text-xs text-muted-foreground">"At drop-off"</figcaption>
                </figure>
                <figure class="flex flex-col gap-1">
                    <img src=bag_src alt=format!("{} bag", d.bag_color) class="aspect-square w-full rounded-2xl border border-border bg-muted object-cover"/>
                    <figcaption class="text-xs text-muted-foreground">"Bag"</figcaption>
                </figure>
            </div>
            <div class="flex min-w-0 flex-1 flex-col gap-3">
                <div class="flex flex-wrap items-start justify-between gap-3">
                    <div>
                        <div class="text-sm text-muted-foreground">"Tag"</div>
                        <div class="text-5xl leading-none font-bold tracking-tight whitespace-nowrap">{d.tag_number.clone()}</div>
                    </div>
                    {score.map(|s| view! {
                        <Badge variant={if confident { BadgeVariant::Default } else { BadgeVariant::Muted }} class="text-sm">
                            {format!("{:.0}% match", s * 100.0)}
                            {(!confident).then(|| " · check carefully")}
                        </Badge>
                    })}
                </div>
                <div class="text-xl font-medium">{d.student_name.clone()}</div>
                <div class="flex flex-wrap gap-x-6 gap-y-1 text-muted-foreground">
                    <ColorSwatch name=d.bag_color.clone() hex=d.bag_color_hex.clone()/>
                    <span>"Since " {d.deposited_local.clone()}</span>
                </div>
                <ActionForm action=hand_over attr:class="mt-auto">
                    <input type="hidden" name="id" value=id.to_string()/>
                    <input type="hidden" name="method" value=method/>
                    {score.map(|s| view! { <input type="hidden" name="score" value=s.to_string()/> })}
                    <Button kind="submit" variant={if highlight { Variant::Default } else { Variant::Outline }} class="w-full @lg:w-auto">
                        "Hand over bag " {d.tag_number}
                    </Button>
                </ActionForm>
            </div>
          </div>
        </Card>
    }
}

/// Shows the result of a hand-over.
#[component]
pub fn HandOverResult(action: ServerAction<HandOverBag>) -> impl IntoView {
    move || {
        action.value().get().map(|r| match r {
            Ok(d) => view! {
                <Alert variant=AlertVariant::Success>
                    {format!("Bag {} handed over to {}. Put tag {} back in the tray.", d.tag_number, d.student_name, d.tag_number)}
                </Alert>
            }
            .into_any(),
            Err(e) => view! { <Alert variant=AlertVariant::Destructive>{err_msg(&e)}</Alert> }.into_any(),
        })
    }
}

/// Default /desk page: everything currently at the desk.
#[component]
pub fn AtDeskPage() -> impl IntoView {
    let (query, set_query) = signal(String::new());
    let hand_over = ServerAction::<HandOverBag>::new();
    let bags = Resource::new(move || (query.get(), hand_over.version().get()), |(q, _)| desk_bags(q));

    view! {
        <PageTitle title="Bags at the desk" subtitle="Newest first. Search by student name or tag number."/>
        <HandOverResult action=hand_over/>
        <input
            class=cn(&[INPUT, "mb-6"])
            type="search"
            aria-label="Search bags"
            placeholder="Name or tag, e.g. Sokha or B-042"
            prop:value=query
            on:input=move |ev| set_query.set(event_target_value(&ev))
        />
        <Transition fallback=|| view! { <p class="text-muted-foreground">"Loading…"</p> }>
            {move || Suspend::new(async move {
                match bags.await {
                    Err(e) => view! { <Alert variant=AlertVariant::Destructive>{err_msg(&e)}</Alert> }.into_any(),
                    Ok((_, list)) if list.is_empty() => view! {
                        <Card tone=Tone::Muted class="p-10 text-center">
                            <p class="text-xl font-medium">"No bags here"</p>
                            <p class="mt-2 text-muted-foreground">"New drop-offs will appear here."</p>
                            <div class="mt-6">
                                <crate::ui::ButtonLink href="/desk/deposit" size=Size::Lg>"Start a drop-off"</crate::ui::ButtonLink>
                            </div>
                        </Card>
                    }
                    .into_any(),
                    Ok((_, list)) => view! {
                        <div class="grid gap-6 xl:grid-cols-2">
                            {list
                                .into_iter()
                                .map(|d| view! { <BagCard deposit=d hand_over=hand_over method="tag"/> })
                                .collect_view()}
                        </div>
                    }
                    .into_any(),
                }
            })}
        </Transition>
    }
}
