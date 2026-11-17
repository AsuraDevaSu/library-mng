use leptos::prelude::*;

use super::{PageTitle, fmt_time};
use crate::{
    admin_fns::{AUDIT_PAGE_SIZE, audit_log},
    auth::err_msg,
    ui::{Alert, AlertVariant, Button, INPUT, Size, TD, TH, TR, Table, TableBody, TableHeader, Variant, cn},
};

#[component]
pub fn AuditPage() -> impl IntoView {
    let (query, set_query) = signal(String::new());
    let (page, set_page) = signal(0i64);
    let entries = Resource::new(move || (query.get(), page.get()), |(q, p)| audit_log(q, p));

    view! {
        <PageTitle title="Audit log" subtitle="Every staff action, newest first."/>
        <input
            class=cn(&[INPUT, "mb-6"])
            type="search"
            aria-label="Filter audit log"
            placeholder="Filter by action (e.g. loan.), entity or staff name"
            prop:value=query
            on:input=move |ev| {
                set_page.set(0);
                set_query.set(event_target_value(&ev));
            }
        />
        <Transition fallback=|| view! { <p class="text-muted-foreground">"Loading…"</p> }>
            {move || Suspend::new(async move {
                match entries.await {
                    Err(e) => view! { <Alert variant=AlertVariant::Destructive>{err_msg(&e)}</Alert> }.into_any(),
                    Ok(list) => {
                        let full_page = list.len() as i64 == AUDIT_PAGE_SIZE;
                        view! {
                            <Table>
                                <TableHeader>
                                    <tr>
                                        <th class=TH>"When"</th>
                                        <th class=TH>"Staff"</th>
                                        <th class=TH>"Action"</th>
                                        <th class=TH>"Details"</th>
                                    </tr>
                                </TableHeader>
                                <TableBody>
                                    {list
                                        .into_iter()
                                        .map(|a| view! {
                                            <tr class=TR>
                                                <td class=cn(&[TD, "whitespace-nowrap"])>{fmt_time(a.created_at)}</td>
                                                <td class=TD>{a.user_name.unwrap_or_else(|| "system".into())}</td>
                                                <td class=TD><code class="rounded-md bg-muted px-2 py-0.5">{a.action}</code></td>
                                                <td class=cn(&[TD, "max-w-md font-mono text-xs break-all text-muted-foreground"])>{a.details}</td>
                                            </tr>
                                        })
                                        .collect_view()}
                                </TableBody>
                            </Table>
                            <div class="mt-6 flex items-center gap-4">
                                <span on:click=move |_| set_page.update(|p| *p = (*p - 1).max(0))>
                                    <Button variant=Variant::Outline size=Size::Sm disabled=Signal::derive(move || page.get() == 0)>"← Newer"</Button>
                                </span>
                                <span class="text-sm text-muted-foreground">"Page " {move || page.get() + 1}</span>
                                <span on:click=move |_| if full_page { set_page.update(|p| *p += 1) }>
                                    <Button variant=Variant::Outline size=Size::Sm disabled=!full_page>"Older →"</Button>
                                </span>
                            </div>
                        }
                        .into_any()
                    }
                }
            })}
        </Transition>
    }
}
