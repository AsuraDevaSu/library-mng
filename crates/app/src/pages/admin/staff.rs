use domain::models::Role;
use leptos::prelude::*;

use super::PageTitle;
use crate::{
    admin_fns::{CreateStaff, ResetStaffPassword, SetStaffActive, UpdateStaffRole, list_staff},
    auth::err_msg,
    ui::{
        Alert, AlertVariant, Badge, BadgeVariant, Button, Card, CardContent, CardHeader, CardTitle, INPUT,
        INPUT_SM, LABEL, Size, TD, TH, TR, Table, TableBody, TableHeader, Variant, cn,
    },
};

fn role_options(selected: Role) -> impl IntoView {
    Role::ALL
        .into_iter()
        .map(|r| view! { <option value=r.as_str() selected=r == selected>{r.label()}</option> })
        .collect_view()
}

#[component]
pub fn StaffPage() -> impl IntoView {
    let create = ServerAction::<CreateStaff>::new();
    let set_role = ServerAction::<UpdateStaffRole>::new();
    let set_active = ServerAction::<SetStaffActive>::new();
    let reset = ServerAction::<ResetStaffPassword>::new();
    let staff = Resource::new(
        move || (create.version().get(), set_role.version().get(), set_active.version().get()),
        |_| list_staff(),
    );

    view! {
        <PageTitle title="Staff" subtitle="Accounts for librarians, desk staff and admins."/>
        {outcome!(set_role, "Role updated.")}
        {outcome!(set_active, "Account updated.")}
        {outcome!(reset, "Password reset. Give the new password to the staff member.")}

        <Transition fallback=|| view! { <p class="text-muted-foreground">"Loading…"</p> }>
            {move || Suspend::new(async move {
                match staff.await {
                    Err(e) => view! { <Alert variant=AlertVariant::Destructive>{err_msg(&e)}</Alert> }.into_any(),
                    Ok(list) => view! {
                        <Table class="mb-10">
                            <TableHeader>
                                <tr>
                                    <th class=TH>"Name"</th>
                                    <th class=TH>"Role"</th>
                                    <th class=TH>"Status"</th>
                                    <th class=TH>"Reset password"</th>
                                </tr>
                            </TableHeader>
                            <TableBody>
                                {list
                                    .into_iter()
                                    .map(|u| {
                                        let id = u.id.to_string();
                                        let (id2, id3) = (id.clone(), id.clone());
                                        view! {
                                            <tr class=TR>
                                                <td class=cn(&[TD, if u.is_active { "" } else { "opacity-50" }])>
                                                    <div class="font-medium">{u.full_name}</div>
                                                    <div class="text-sm text-muted-foreground">{u.email}</div>
                                                </td>
                                                <td class=TD>
                                                    <ActionForm action=set_role attr:class="flex items-center gap-2">
                                                        <input type="hidden" name="id" value=id.clone()/>
                                                        <select name="role" aria-label="Role" class=INPUT_SM>{role_options(u.role)}</select>
                                                        <Button kind="submit" variant=Variant::Outline size=Size::Sm>"Save"</Button>
                                                    </ActionForm>
                                                </td>
                                                <td class=TD>
                                                    <ActionForm action=set_active attr:class="flex items-center gap-2">
                                                        <input type="hidden" name="id" value=id2.clone()/>
                                                        <input type="hidden" name="active" value=(!u.is_active).to_string()/>
                                                        {if u.is_active {
                                                            view! { <Badge variant=BadgeVariant::Brand>"Active"</Badge> }.into_any()
                                                        } else {
                                                            view! { <Badge variant=BadgeVariant::Muted>"Disabled"</Badge> }.into_any()
                                                        }}
                                                        <Button kind="submit" variant=Variant::Ghost size=Size::Sm>
                                                            {if u.is_active { "Disable" } else { "Enable" }}
                                                        </Button>
                                                    </ActionForm>
                                                </td>
                                                <td class=TD>
                                                    <ActionForm action=reset attr:class="flex items-center gap-2">
                                                        <input type="hidden" name="id" value=id3.clone()/>
                                                        <input
                                                            class=cn(&[INPUT_SM, "w-36"])
                                                            type="password"
                                                            name="password"
                                                            aria-label="New password"
                                                            placeholder="New password"
                                                            minlength="8"
                                                            required=true
                                                            autocomplete="new-password"
                                                        />
                                                        <Button kind="submit" variant=Variant::Outline size=Size::Sm>"Reset"</Button>
                                                    </ActionForm>
                                                </td>
                                            </tr>
                                        }
                                    })
                                    .collect_view()}
                            </TableBody>
                        </Table>
                    }
                    .into_any(),
                }
            })}
        </Transition>

        <Card tone=crate::ui::Tone::Muted>
            <CardHeader><CardTitle>"Add staff"</CardTitle></CardHeader>
            <CardContent>
                {outcome!(create, "Staff account created.")}
                <ActionForm action=create attr:class="grid items-end gap-4 md:grid-cols-2 xl:grid-cols-4">
                    <label class=LABEL>"Full name" <input class=INPUT name="input[full_name]" required=true/></label>
                    <label class=LABEL>"Email" <input class=INPUT type="email" name="input[email]" required=true/></label>
                    <label class=LABEL>"Role" <select class=INPUT name="input[role]">{role_options(Role::Librarian)}</select></label>
                    <label class=LABEL>
                        "Initial password"
                        <input class=INPUT type="password" name="input[password]" minlength="8" required=true autocomplete="new-password"/>
                    </label>
                    <div class="md:col-span-2 xl:col-span-4">
                        <Button kind="submit" variant=Variant::Brand>"Create account"</Button>
                    </div>
                </ActionForm>
            </CardContent>
        </Card>
    }
}
