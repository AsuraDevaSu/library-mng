use domain::{dto::{BAG_COLORS, DepositInput}, models::BagDeposit};
use leptos::prelude::*;

use super::ColorSwatch;
use crate::{
    auth::err_msg,
    camera::Camera,
    desk_fns::DepositBag,
    pages::admin::PageTitle,
    ui::{Alert, AlertVariant, Button, Card, CardContent, CardHeader, CardTitle, INPUT, LABEL, Size, Tone, Variant, cn},
};

#[derive(Clone, Copy, PartialEq)]
enum Step {
    Face,
    Bag,
    Ready,
}

#[component]
pub fn DepositPage() -> impl IntoView {
    let name = RwSignal::new(String::new());
    let card = RwSignal::new(String::new());
    let consent = RwSignal::new(true);
    let color = RwSignal::new(String::new()); // "" = detect automatically
    let face = RwSignal::new(None::<String>);
    let bag = RwSignal::new(None::<String>);
    let receipt = RwSignal::new(None::<BagDeposit>);

    let step = Memo::new(move |_| {
        if consent.get() && face.with(Option::is_none) {
            Step::Face
        } else if bag.with(Option::is_none) {
            Step::Bag
        } else {
            Step::Ready
        }
    });

    let save = ServerAction::<DepositBag>::new();
    let reset = move || {
        name.set(String::new());
        card.set(String::new());
        consent.set(true);
        color.set(String::new());
        face.set(None);
        bag.set(None);
    };
    Effect::new(move |_| {
        if let Some(Ok(d)) = save.value().get() {
            receipt.set(Some(d));
            reset();
        }
    });

    let on_capture = Callback::new(move |url: String| match step.get_untracked() {
        Step::Face => face.set(Some(url)),
        _ => bag.set(Some(url)),
    });
    let capture_label = Signal::derive(move || match step.get() {
        Step::Face => "① Take face photo".to_string(),
        Step::Bag => "② Take bag photo".to_string(),
        Step::Ready => "Retake bag photo".to_string(),
    });
    let can_save = move || {
        !name.with(|n| n.trim().is_empty()) && step.get() == Step::Ready && !save.pending().get()
    };
    let submit = move |_| {
        save.dispatch(DepositBag {
            input: DepositInput {
                student_name: name.get_untracked(),
                card_number: Some(card.get_untracked()).filter(|c| !c.trim().is_empty()),
                photo_consent: consent.get_untracked(),
                face_photo: if consent.get_untracked() { face.get_untracked() } else { None },
                bag_photo: bag.get_untracked().unwrap_or_default(),
                bag_color: Some(color.get_untracked()).filter(|c| !c.is_empty()),
            },
        });
    };

    view! {
        <PageTitle title="Drop-off" subtitle="For every student and teacher, no membership needed. Name → face photo → bag photo → attach the tag."/>
        <Show
            when=move || receipt.with(Option::is_some)
            fallback=move || view! {
                <div class="grid gap-8 lg:grid-cols-[minmax(0,1.2fr)_minmax(0,1fr)]">
                    <div class="flex flex-col gap-4">
                        <StepIndicator step=step consent=consent/>
                        <Camera capture_label=capture_label on_capture=on_capture mirror=Signal::derive(move || step.get() == Step::Face)/>
                    </div>

                    <Card>
                        <CardHeader><CardTitle>"Student & bag"</CardTitle></CardHeader>
                        <CardContent class="flex flex-col gap-5">
                            {move || save.value().get().and_then(Result::err).map(|e| view! {
                                <Alert variant=AlertVariant::Destructive>{err_msg(&e)}</Alert>
                            })}
                            <label class=LABEL>
                                "Name (student or teacher)"
                                <input class=INPUT required=true autocomplete="off" prop:value=name
                                    on:input=move |ev| name.set(event_target_value(&ev))/>
                            </label>
                            <label class=LABEL>
                                "ID card number (optional)"
                                <input class=INPUT autocomplete="off" placeholder="Student or teacher card. No library membership needed" prop:value=card
                                    on:input=move |ev| card.set(event_target_value(&ev))/>
                            </label>
                            <label class="flex items-start gap-3 text-sm">
                                <input type="checkbox" class="mt-1 size-5 accent-[#191A23]" prop:checked=consent
                                    on:change=move |ev| consent.set(event_target_checked(&ev))/>
                                <span>
                                    <span class="font-medium">"Student agrees to a face photo"</span><br/>
                                    <span class="text-muted-foreground">"Used only to find the bag at pickup, deleted afterwards. Untick to use the tag number only."</span>
                                </span>
                            </label>

                            <div class="grid grid-cols-2 gap-3">
                                <Thumb label="Face" src=face enabled=consent/>
                                <Thumb label="Bag" src=bag enabled=Signal::derive(|| true)/>
                            </div>

                            <label class=LABEL>
                                "Bag colour"
                                <select class=INPUT on:change=move |ev| color.set(event_target_value(&ev))>
                                    <option value="" selected=move || color.with(String::is_empty)>"Detect from photo"</option>
                                    {BAG_COLORS.iter().map(|(n, _)| view! {
                                        <option value=*n selected=move || color.get() == *n>{*n}</option>
                                    }).collect_view()}
                                </select>
                            </label>

                            <Button variant=Variant::Default size=Size::Lg disabled=Signal::derive(move || !can_save())
                                class="w-full" on:click=submit>
                                {move || if save.pending().get() { "Saving…" } else { "Save & assign tag" }}
                            </Button>
                        </CardContent>
                    </Card>
                </div>
            }
        >
            {move || receipt.get().map(|d| view! {
                <Card tone=Tone::Brand large=true class="mx-auto flex max-w-2xl flex-col items-center gap-6 p-10 text-center">
                    <p class="text-lg">"Attach this tag to the bag"</p>
                    <div class="text-8xl leading-none font-bold tracking-tight md:text-9xl">{d.tag_number.clone()}</div>
                    <div class="flex flex-wrap items-center justify-center gap-x-6 gap-y-2 text-lg">
                        <span class="font-medium">{d.student_name.clone()}</span>
                        <ColorSwatch name=d.bag_color.clone() hex=d.bag_color_hex.clone()/>
                        <span>"at " {d.deposited_local.clone()}</span>
                    </div>
                    {(!d.photo_consent).then(|| view! {
                        <p class="text-sm">"No face photo: at pickup, find this bag by tag number or name."</p>
                    })}
                    <button type="button" class=cn(&[&crate::ui::button_class(Variant::Default, Size::Lg)]) on:click=move |_| receipt.set(None)>
                        "Next student"
                    </button>
                </Card>
            })}
        </Show>
    }
}

#[component]
fn StepIndicator(step: Memo<Step>, consent: RwSignal<bool>) -> impl IntoView {
    let pill = move |active: bool, done: bool| {
        cn(&[
            "rounded-full border border-border px-3 py-1 text-sm",
            if active { "bg-accent font-medium" } else if done { "bg-primary text-primary-foreground" } else { "bg-background" },
        ])
    };
    view! {
        <div class="flex flex-wrap gap-2">
            <Show when=move || consent.get()>
                <span class=move || pill(step.get() == Step::Face, step.get() != Step::Face)>"① Face"</span>
            </Show>
            <span class=move || pill(step.get() == Step::Bag, step.get() == Step::Ready)>"② Bag"</span>
            <span class=move || pill(step.get() == Step::Ready, false)>"③ Save"</span>
        </div>
    }
}

/// Captured photo preview with a "Retake" button that clears it.
#[component]
fn Thumb(label: &'static str, src: RwSignal<Option<String>>, #[prop(into)] enabled: Signal<bool>) -> impl IntoView {
    view! {
        <div class="flex flex-col gap-1">
            <div class="relative aspect-square overflow-hidden rounded-2xl border border-border bg-muted">
                {move || match (enabled.get(), src.get()) {
                    (false, _) => view! { <div class="grid h-full place-items-center p-2 text-center text-xs text-muted-foreground">"Not needed"</div> }.into_any(),
                    (true, None) => view! { <div class="grid h-full place-items-center text-sm text-muted-foreground">{label} " photo"</div> }.into_any(),
                    (true, Some(url)) => view! { <img src=url alt=format!("{label} photo") class="h-full w-full object-cover"/> }.into_any(),
                }}
            </div>
            <Show when=move || enabled.get() && src.with(Option::is_some)>
                <button type="button" class="self-start text-sm underline underline-offset-4" on:click=move |_| src.set(None)>
                    "Retake " {label}
                </button>
            </Show>
        </div>
    }
}
