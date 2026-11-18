use leptos::prelude::*;

use super::{BagCard, HandOverResult};
use crate::{
    auth::err_msg,
    camera::Camera,
    desk_fns::{HandOverBag, ScanFace, desk_bags},
    pages::admin::PageTitle,
    ui::{Alert, AlertVariant, Card, CardContent, CardHeader, CardTitle, INPUT, Tone},
};

#[component]
pub fn PickupPage() -> impl IntoView {
    let scan = ServerAction::<ScanFace>::new();
    let hand_over = ServerAction::<HandOverBag>::new();
    // Hide scan results once a bag from them has been handed over.
    let shown_scan = RwSignal::new(0usize);
    Effect::new(move |_| {
        if let Some(Ok(_)) = hand_over.value().get() {
            shown_scan.set(scan.version().get_untracked());
        }
    });

    let (query, set_query) = signal(String::new());
    let search = Resource::new(
        move || (query.get(), hand_over.version().get()),
        |(q, _)| async move {
            if q.trim().is_empty() { Ok(None) } else { desk_bags(q).await.map(|(_, list)| Some(list)) }
        },
    );

    let on_capture = Callback::new(move |photo: String| {
        scan.dispatch(ScanFace { photo });
    });

    view! {
        <PageTitle title="Pickup" subtitle="Scan the student's face, compare with the drop-off photo, hand over the bag."/>
        <HandOverResult action=hand_over/>
        <div class="grid gap-8 lg:grid-cols-[minmax(0,1fr)_minmax(0,1.3fr)]">
            <div class="flex flex-col gap-6">
                <Camera
                    capture_label=Signal::derive(move || if scan.pending().get() { "Scanning…".to_string() } else { "Scan face".to_string() })
                    on_capture=on_capture
                    mirror=true
                    disabled=Signal::derive(move || scan.pending().get())
                />
                <Card tone=Tone::Muted>
                    <CardHeader><CardTitle>"No match? Search instead"</CardTitle></CardHeader>
                    <CardContent>
                        <input
                            class=INPUT
                            type="search"
                            aria-label="Search by name or tag"
                            placeholder="Name, tag or ID card, e.g. B-042"
                            prop:value=query
                            on:input=move |ev| set_query.set(event_target_value(&ev))
                        />
                    </CardContent>
                </Card>
            </div>

            <div class="flex min-w-0 flex-col gap-6">
                // Face scan results
                {move || {
                    if scan.version().get() == shown_scan.get() {
                        return None;
                    }
                    scan.value().get().map(|r| match r {
                        Err(e) => view! { <Alert variant=AlertVariant::Destructive>{err_msg(&e)}</Alert> }.into_any(),
                        Ok(res) if res.faces_found == 0 => view! {
                            <Alert variant=AlertVariant::Info>"No face found. Ask the student to look at the camera and scan again."</Alert>
                        }.into_any(),
                        Ok(res) if res.matches.is_empty() => view! {
                            <Alert variant=AlertVariant::Info>"No bags with face photos are at the desk. Search by name or tag."</Alert>
                        }.into_any(),
                        Ok(res) => {
                            let best_confident = res.matches.first().is_some_and(|m| m.confident);
                            view! {
                                {(!best_confident).then(|| view! {
                                    <Alert variant=AlertVariant::Info>"No confident match. Compare the photos carefully or search by name or tag."</Alert>
                                })}
                                {res.matches
                                    .into_iter()
                                    .enumerate()
                                    .map(|(i, m)| view! {
                                        <BagCard deposit=m.deposit hand_over=hand_over method="face"
                                            score=m.score confident=m.confident highlight={i == 0 && m.confident}/>
                                    })
                                    .collect_view()}
                            }
                            .into_any()
                        }
                    })
                }}

                // Name / tag search results
                <Transition fallback=|| ()>
                    {move || Suspend::new(async move {
                        match search.await {
                            Ok(None) => ().into_any(),
                            Ok(Some(list)) if list.is_empty() => view! {
                                <Alert variant=AlertVariant::Info>"No bag at the desk matches that name or tag."</Alert>
                            }.into_any(),
                            Ok(Some(list)) => list
                                .into_iter()
                                .map(|d| view! { <BagCard deposit=d hand_over=hand_over method="name"/> })
                                .collect_view()
                                .into_any(),
                            Err(e) => view! { <Alert variant=AlertVariant::Destructive>{err_msg(&e)}</Alert> }.into_any(),
                        }
                    })}
                </Transition>
            </div>
        </div>
    }
}
