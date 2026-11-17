use leptos::prelude::*;

use super::{PageTitle, fmt_time};
use crate::{
    admin_fns::{SaveSettings, get_settings},
    auth::err_msg,
    ui::{Alert, AlertVariant, Button, Card, CardContent, CardDescription, CardHeader, CardTitle, INPUT, LABEL, Variant},
};

#[component]
pub fn SettingsPage() -> impl IntoView {
    let save = ServerAction::<SaveSettings>::new();
    let settings = Resource::new(move || save.version().get(), |_| get_settings());

    view! {
        <PageTitle title="Settings"/>
        {outcome!(save, "Settings saved.")}
        <Transition fallback=|| view! { <p class="text-muted-foreground">"Loading…"</p> }>
            {move || Suspend::new(async move {
                match settings.await {
                    Err(e) => view! { <Alert variant=AlertVariant::Destructive>{err_msg(&e)}</Alert> }.into_any(),
                    Ok(s) => view! {
                        <ActionForm action=save attr:class="flex flex-col gap-6">
                            <Card>
                                <CardHeader>
                                    <CardTitle>"Library"</CardTitle>
                                    <CardDescription>"Shown on the public website."</CardDescription>
                                </CardHeader>
                                <CardContent class="grid gap-4 md:grid-cols-2">
                                    <label class=LABEL>"Library name" <input class=INPUT name="input[library_name]" value=s.library_name required=true/></label>
                                    <label class=LABEL>"Opening hours" <input class=INPUT name="input[opening_hours]" value=s.opening_hours/></label>
                                </CardContent>
                            </Card>
                            <Card>
                                <CardHeader>
                                    <CardTitle>"Bag deposit desk"</CardTitle>
                                    <CardDescription>"Privacy and matching settings for face & bag photos."</CardDescription>
                                </CardHeader>
                                <CardContent class="grid gap-4 md:grid-cols-2">
                                    <label class=LABEL>
                                        "Keep photos for (days after pickup)"
                                        <input class=INPUT type="number" min="1" max="365" name="input[bag_photo_retention_days]"
                                            value=s.bag_photo_retention_days.to_string() required=true/>
                                    </label>
                                    <label class=LABEL>
                                        "Face match threshold (0–1, higher = stricter)"
                                        <input class=INPUT type="number" min="0" max="1" step="0.001" name="input[face_match_threshold]"
                                            value=s.face_match_threshold.to_string() required=true/>
                                    </label>
                                </CardContent>
                            </Card>
                            <div class="flex flex-wrap items-center gap-4">
                                <Button kind="submit" variant=Variant::Brand>"Save settings"</Button>
                                <span class="text-sm text-muted-foreground">"Last changed " {fmt_time(s.updated_at)}</span>
                            </div>
                        </ActionForm>
                    }
                    .into_any(),
                }
            })}
        </Transition>
    }
}
