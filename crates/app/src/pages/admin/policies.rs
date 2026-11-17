use domain::models::MemberType;
use leptos::prelude::*;

use super::PageTitle;
use crate::{
    admin_fns::{CreateMemberType, UpdateMemberType, list_member_types},
    auth::err_msg,
    ui::{Alert, AlertVariant, Button, Card, CardContent, CardHeader, CardTitle, INPUT, LABEL, Tone, Variant},
};

/// Input fields shared by the "edit" and "add" forms (names match `MemberTypeInput`).
fn policy_fields(t: Option<MemberType>) -> impl IntoView {
    let v = |f: fn(&MemberType) -> String| t.as_ref().map(f).unwrap_or_default();
    view! {
        <label class=LABEL>"Name" <input class=INPUT name="input[name]" value=v(|t| t.name.clone()) required=true/></label>
        <label class=LABEL>"Max books" <input class=INPUT type="number" min="0" name="input[max_loans]" value=v(|t| t.max_loans.to_string()) required=true/></label>
        <label class=LABEL>"Loan days" <input class=INPUT type="number" min="1" name="input[loan_days]" value=v(|t| t.loan_days.to_string()) required=true/></label>
        <label class=LABEL>"Max renewals" <input class=INPUT type="number" min="0" name="input[max_renewals]" value=v(|t| t.max_renewals.to_string()) required=true/></label>
        <label class=LABEL>"Fine per day" <input class=INPUT type="number" min="0" step="0.01" name="input[daily_fine]" value=v(|t| t.daily_fine.to_string()) required=true/></label>
        <label class=LABEL>"Max fine per loan" <input class=INPUT type="number" min="0" step="0.01" name="input[max_fine]" value=v(|t| t.max_fine.to_string()) required=true/></label>
        <label class=LABEL>"Block borrowing above" <input class=INPUT type="number" min="0" step="0.01" name="input[fine_block_threshold]" value=v(|t| t.fine_block_threshold.to_string()) required=true/></label>
    }
}

const GRID: &str = "grid items-end gap-4 sm:grid-cols-2 xl:grid-cols-4";

#[component]
pub fn PoliciesPage() -> impl IntoView {
    let create = ServerAction::<CreateMemberType>::new();
    let update = ServerAction::<UpdateMemberType>::new();
    let types = Resource::new(move || (create.version().get(), update.version().get()), |_| list_member_types());

    view! {
        <PageTitle title="Borrowing rules" subtitle="Changes apply to new loans. Existing due dates are not changed."/>
        {outcome!(update, "Saved.")}
        <Transition fallback=|| view! { <p class="text-muted-foreground">"Loading…"</p> }>
            {move || Suspend::new(async move {
                match types.await {
                    Err(e) => view! { <Alert variant=AlertVariant::Destructive>{err_msg(&e)}</Alert> }.into_any(),
                    Ok(list) => view! {
                        <div class="mb-10 flex flex-col gap-6">
                            {list
                                .into_iter()
                                .map(|t| {
                                    let (title, label, id) = (t.name.clone(), t.name.clone(), t.id.to_string());
                                    view! {
                                    <Card>
                                        <CardHeader>
                                            <CardTitle class="capitalize">{title}</CardTitle>
                                        </CardHeader>
                                        <CardContent>
                                            <ActionForm action=update attr:class=GRID>
                                                <input type="hidden" name="id" value=id.clone()/>
                                                {policy_fields(Some(t.clone()))}
                                                <Button kind="submit">"Save " {label.clone()}</Button>
                                            </ActionForm>
                                        </CardContent>
                                    </Card>
                                    }
                                })
                                .collect_view()}
                        </div>
                    }
                    .into_any(),
                }
            })}
        </Transition>

        <Card tone=Tone::Muted>
            <CardHeader><CardTitle>"Add a member type"</CardTitle></CardHeader>
            <CardContent>
                {outcome!(create, "Member type added.")}
                <ActionForm action=create attr:class=GRID>
                    {policy_fields(None)}
                    <Button kind="submit" variant=Variant::Brand>"Add"</Button>
                </ActionForm>
            </CardContent>
        </Card>
    }
}
