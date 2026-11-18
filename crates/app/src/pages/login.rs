use leptos::prelude::*;

use crate::{
    auth::AuthCtx,
    ui::{Button, Card, CardContent, CardDescription, CardHeader, Highlight, INPUT, LABEL, Size},
};

#[component]
pub fn LoginPage() -> impl IntoView {
    let auth = expect_context::<AuthCtx>();
    view! {
        <crate::pages::admin::StaffBar/>
        <div class="mx-auto max-w-md px-4 py-16">
            <Card class="bg-card">
                <CardHeader class="p-8 pb-4">
                    <h1 class="text-3xl font-medium"><Highlight>"Staff login"</Highlight></h1>
                    <CardDescription class="mt-2 text-base">"For librarians, desk staff and admins. Students and teachers don't need an account — the catalog is open to everyone."</CardDescription>
                </CardHeader>
                <CardContent class="px-8 pb-8">
                    {outcome!(auth.login, "")}
                    <ActionForm action=auth.login attr:class="flex flex-col gap-5">
                        <label class=LABEL>
                            "Email"
                            <input class=INPUT type="email" name="email" required=true autocomplete="username"/>
                        </label>
                        <label class=LABEL>
                            "Password"
                            <input class=INPUT type="password" name="password" required=true autocomplete="current-password"/>
                        </label>
                        <Button kind="submit" size=Size::Lg disabled=Signal::derive(move || auth.login.pending().get())>
                            "Log in"
                        </Button>
                    </ActionForm>
                </CardContent>
            </Card>
        </div>
    }
}
