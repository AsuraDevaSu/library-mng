use leptos::prelude::*;

use super::{PageTitle, fmt_time};
use crate::{
    admin_fns::admin_dashboard,
    auth::err_msg,
    ui::{Alert, AlertVariant, Card, CardContent, CardHeader, CardTitle, Tone, cn},
};

#[component]
pub fn DashboardPage() -> impl IntoView {
    let data = Resource::new(|| (), |_| admin_dashboard());
    view! {
        // Librarians manage books but don't see the admin dashboard.
        <super::RequireRole role=domain::models::Role::Admin fallback="/admin/books"/>
        <PageTitle title="Dashboard" subtitle="Today at a glance."/>
        <Suspense fallback=|| view! { <p class="text-muted-foreground">"Loading…"</p> }>
            {move || Suspend::new(async move {
                match data.await {
                    Err(e) => view! { <Alert variant=AlertVariant::Destructive>{err_msg(&e)}</Alert> }.into_any(),
                    Ok((s, recent)) => view! {
                        <div class="mb-10 grid grid-cols-2 gap-5 md:grid-cols-3 xl:grid-cols-4">
                            <Stat label="Book titles" value=s.titles.to_string() tone=Tone::Muted/>
                            <Stat label="Copies" value=s.copies.to_string() tone=Tone::Muted/>
                            <Stat label="Active members" value=s.active_members.to_string() tone=Tone::Brand/>
                            <Stat label="On loan" value=s.open_loans.to_string() tone=Tone::Plain/>
                            <Stat label="Overdue" value=s.overdue_loans.to_string()
                                tone={if s.overdue_loans > 0 { Tone::Dark } else { Tone::Plain }}/>
                            <Stat label="Unpaid fines" value=s.unpaid_fines.to_string() tone=Tone::Plain/>
                            <Stat label="Active staff" value=s.active_staff.to_string() tone=Tone::Muted/>
                        </div>
                        <Card>
                            <CardHeader><CardTitle>"Recent activity"</CardTitle></CardHeader>
                            <CardContent>
                                <ul class="divide-y divide-hairline">
                                    {recent
                                        .into_iter()
                                        .map(|a| view! {
                                            <li class="flex flex-wrap items-center gap-x-3 gap-y-1 py-3">
                                                <code class="rounded-md bg-muted px-2 py-0.5 text-sm">{a.action}</code>
                                                <span>{a.user_name.unwrap_or_else(|| "system".into())}</span>
                                                <span class="ml-auto text-sm text-muted-foreground">{fmt_time(a.created_at)}</span>
                                            </li>
                                        })
                                        .collect_view()}
                                </ul>
                            </CardContent>
                        </Card>
                    }
                    .into_any(),
                }
            })}
        </Suspense>
    }
}

#[component]
fn Stat(label: &'static str, value: String, tone: Tone) -> impl IntoView {
    let label_class = if tone == Tone::Dark { "text-white/70" } else { "text-muted-foreground" };
    view! {
        <Card tone=tone class="p-5">
            <div class="text-4xl font-medium tabular-nums">{value}</div>
            <div class=cn(&["mt-1 text-sm", if tone == Tone::Brand { "text-accent-foreground" } else { label_class }])>{label}</div>
        </Card>
    }
}
