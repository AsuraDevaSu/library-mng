//! Desk camera: off by default; turns on only to take a photo (and off right
//! after, or after a minute idle). JPEG capture, plus a photo-file
//! fallback (phones/tablets over plain HTTP, or no webcam). Browser-only code
//! is behind the `hydrate` feature; on the server this renders an empty frame.

use leptos::{html, prelude::*};

use crate::ui::{Size, Variant, button_class};

/// The camera switches itself off if no photo is taken within this time.
#[cfg(feature = "hydrate")]
const AUTO_OFF_SECS: u64 = 60;

#[component]
pub fn Camera(
    /// Text on the capture button, e.g. "Take face photo".
    #[prop(into)]
    capture_label: Signal<String>,
    /// Receives a `data:image/jpeg;base64,…` URL.
    on_capture: Callback<String>,
    /// Mirror the preview (natural for selfies). Captured photos are never mirrored.
    #[prop(optional, into)]
    mirror: Signal<bool>,
    #[prop(optional, into)] disabled: Signal<bool>,
) -> impl IntoView {
    let video = NodeRef::<html::Video>::new();
    let (error, set_error) = signal(None::<String>);
    // The camera is off by default and only on while a photo is being taken.
    let (live, set_live) = signal(false);
    let (starting, set_starting) = signal(false);
    // Bumped on every close, so an old auto-off timer can't close a newer session.
    let session = StoredValue::new(0u32);

    #[cfg(feature = "hydrate")]
    let stream = StoredValue::new_local(None::<web_sys::MediaStream>);

    let close = move || {
        #[cfg(feature = "hydrate")]
        {
            if let Some(s) = stream.try_get_value().flatten() {
                imp::stop(&s);
            }
            let _ = stream.try_update_value(|s| *s = None);
            if let Some(v) = video.get_untracked() {
                v.set_src_object(None);
            }
        }
        session.update_value(|g| *g += 1);
        set_live.set(false);
    };

    let open = move || {
        if live.get_untracked() || starting.get_untracked() {
            return;
        }
        set_error.set(None);
        #[cfg(feature = "hydrate")]
        if let Some(v) = video.get_untracked() {
            set_starting.set(true);
            leptos::task::spawn_local(async move {
                let result = imp::start(&v).await;
                set_starting.set(false);
                match result {
                    Ok(s) => {
                        stream.set_value(Some(s));
                        set_live.set(true);
                        // Turn the camera off if nobody takes a photo within a minute.
                        let this_session = session.get_value();
                        set_timeout(
                            move || {
                                if session.try_get_value() == Some(this_session) {
                                    close();
                                }
                            },
                            std::time::Duration::from_secs(AUTO_OFF_SECS),
                        );
                    }
                    Err(e) => set_error.set(Some(e)),
                }
            });
        }
    };

    // Leaving the page always releases the camera.
    #[cfg(feature = "hydrate")]
    on_cleanup(move || {
        if let Some(s) = stream.try_get_value().flatten() {
            imp::stop(&s);
        }
    });
    #[cfg(not(feature = "hydrate"))]
    let _ = (set_starting, on_capture);

    let capture = move |_| {
        #[cfg(feature = "hydrate")]
        if let Some(v) = video.get_untracked() {
            match imp::capture(&v) {
                Some(url) => {
                    close();
                    on_capture.run(url);
                }
                None => set_error.set(Some("The camera isn't ready yet.".into())),
            }
        }
    };

    let on_file = move |ev: leptos::ev::Event| {
        #[cfg(feature = "hydrate")]
        {
            let input: web_sys::HtmlInputElement = event_target(&ev);
            if let Some(file) = input.files().and_then(|l| l.get(0)) {
                leptos::task::spawn_local(async move {
                    match imp::read_file(file).await {
                        Some(url) => on_capture.run(url),
                        None => set_error.set(Some("That file couldn't be read as a photo.".into())),
                    }
                });
            }
            input.set_value("");
        }
        #[cfg(not(feature = "hydrate"))]
        let _ = ev;
    };

    view! {
        <div class="flex flex-col gap-3">
            <div class="relative aspect-[4/3] overflow-hidden rounded-3xl border border-border bg-primary shadow-brut">
                <video
                    node_ref=video
                    autoplay=true
                    playsinline=true
                    muted=true
                    class="h-full w-full object-cover"
                    class:invisible=move || !live.get()
                    class:-scale-x-100=move || mirror.get()
                ></video>
                {move || live.get().then(|| view! {
                    <span class="absolute top-3 left-3 inline-flex items-center gap-1.5 rounded-full bg-destructive px-2.5 py-1 text-xs font-medium text-destructive-foreground">
                        <span class="size-2 rounded-full bg-white"></span>
                        "Camera on"
                    </span>
                })}
                {move || (!live.get()).then(|| {
                    let (title, detail) = if starting.get() {
                        ("Starting camera…".to_string(), String::new())
                    } else if let Some(e) = error.get() {
                        ("Camera unavailable".to_string(), e)
                    } else {
                        ("Camera is off".to_string(), "It turns on only while you take a photo.".to_string())
                    };
                    view! {
                        <div class="absolute inset-0 flex flex-col items-center justify-center gap-2 p-8 text-center text-white/90">
                            <svg class="size-10 text-white/60" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"
                                stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                                <path d="M4 8h3l2-3h6l2 3h3v11H4z"/><circle cx="12" cy="13" r="3.5"/>
                            </svg>
                            <p class="font-medium">{title}</p>
                            <p class="text-sm text-white/70">{detail}</p>
                        </div>
                    }
                })}
            </div>
            <div class="flex flex-wrap gap-3">
                <Show
                    when=move || live.get()
                    fallback=move || view! {
                        <button
                            type="button"
                            class=button_class(Variant::Brand, Size::Lg)
                            disabled=move || disabled.get() || starting.get()
                            on:click=move |_| open()
                        >
                            {move || capture_label.get()}
                        </button>
                    }
                >
                    <button type="button" class=button_class(Variant::Brand, Size::Lg) on:click=capture>
                        "Capture photo"
                    </button>
                    <button type="button" class=button_class(Variant::Outline, Size::Lg) on:click=move |_| close()>
                        "Cancel"
                    </button>
                </Show>
                <label class=format!("{} cursor-pointer", button_class(Variant::Outline, Size::Lg))>
                    "Use a photo file"
                    <input type="file" accept="image/*" class="sr-only" on:change=on_file/>
                </label>
            </div>
        </div>
    }
}

#[cfg(feature = "hydrate")]
mod imp {
    use wasm_bindgen::{JsCast, JsValue, closure::Closure};
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{
        CanvasRenderingContext2d, HtmlCanvasElement, HtmlImageElement, HtmlVideoElement, MediaStream,
        MediaStreamConstraints, MediaStreamTrack,
    };

    /// Longest side of captured photos; keeps uploads around 100–200 KB.
    const MAX_SIDE: f64 = 960.0;

    pub async fn start(video: &HtmlVideoElement) -> Result<MediaStream, String> {
        let window = web_sys::window().ok_or("no window")?;
        let devices = window.navigator().media_devices().map_err(|_| {
            "The camera is only available over HTTPS or on localhost. Use “Use a photo file” instead.".to_string()
        })?;
        let constraints = MediaStreamConstraints::new();
        let video_opts = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&video_opts, &"width".into(), &JsValue::from_f64(1280.0));
        let _ = js_sys::Reflect::set(&video_opts, &"height".into(), &JsValue::from_f64(960.0));
        constraints.set_video(&video_opts);
        constraints.set_audio(&JsValue::FALSE);
        let promise = devices
            .get_user_media_with_constraints(&constraints)
            .map_err(|_| "This browser can't open the camera.".to_string())?;
        let stream: MediaStream = JsFuture::from(promise)
            .await
            .map_err(|_| "Camera blocked or not connected. Allow camera access, or use “Use a photo file”.".to_string())?
            .unchecked_into();
        video.set_src_object(Some(&stream));
        let _ = video.play();
        Ok(stream)
    }

    pub fn stop(stream: &MediaStream) {
        for t in stream.get_tracks().iter() {
            t.unchecked_into::<MediaStreamTrack>().stop();
        }
    }

    fn to_jpeg(draw: impl FnOnce(&CanvasRenderingContext2d, f64, f64), w: f64, h: f64) -> Option<String> {
        if w <= 0.0 || h <= 0.0 {
            return None;
        }
        let scale = (MAX_SIDE / w.max(h)).min(1.0);
        let (cw, ch) = ((w * scale).round(), (h * scale).round());
        let canvas: HtmlCanvasElement =
            web_sys::window()?.document()?.create_element("canvas").ok()?.unchecked_into();
        canvas.set_width(cw as u32);
        canvas.set_height(ch as u32);
        let ctx: CanvasRenderingContext2d = canvas.get_context("2d").ok()??.unchecked_into();
        draw(&ctx, cw, ch);
        canvas.to_data_url_with_type_and_encoder_options("image/jpeg", &JsValue::from_f64(0.85)).ok()
    }

    pub fn capture(video: &HtmlVideoElement) -> Option<String> {
        let (w, h) = (video.video_width() as f64, video.video_height() as f64);
        to_jpeg(|ctx, cw, ch| { let _ = ctx.draw_image_with_html_video_element_and_dw_and_dh(video, 0.0, 0.0, cw, ch); }, w, h)
    }

    /// Reads a picked file and downscales it like a camera capture.
    pub async fn read_file(file: web_sys::File) -> Option<String> {
        let reader = web_sys::FileReader::new().ok()?;
        let r = reader.clone();
        let loaded = js_sys::Promise::new(&mut |resolve, reject| {
            r.set_onload(Some(Closure::once_into_js(move || { let _ = resolve.call0(&JsValue::NULL); }).unchecked_ref()));
            r.set_onerror(Some(Closure::once_into_js(move || { let _ = reject.call0(&JsValue::NULL); }).unchecked_ref()));
        });
        reader.read_as_data_url(&file).ok()?;
        JsFuture::from(loaded).await.ok()?;
        let src = reader.result().ok()?.as_string()?;

        let img = HtmlImageElement::new().ok()?;
        let i = img.clone();
        let decoded = js_sys::Promise::new(&mut |resolve, reject| {
            i.set_onload(Some(Closure::once_into_js(move || { let _ = resolve.call0(&JsValue::NULL); }).unchecked_ref()));
            i.set_onerror(Some(Closure::once_into_js(move || { let _ = reject.call0(&JsValue::NULL); }).unchecked_ref()));
        });
        img.set_src(&src);
        JsFuture::from(decoded).await.ok()?;
        let (w, h) = (img.natural_width() as f64, img.natural_height() as f64);
        to_jpeg(|ctx, cw, ch| { let _ = ctx.draw_image_with_html_image_element_and_dw_and_dh(&img, 0.0, 0.0, cw, ch); }, w, h)
    }
}
