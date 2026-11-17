//! shadcn/ui for Leptos.
//!
//! shadcn/ui is React-only, so this module ports its design system: the same
//! token names (see `style/tailwind.css`) and the same component recipes
//! (Button variants/sizes, Card, Badge, Alert, Table, Input…), written as
//! Leptos components and Tailwind class constants. Themed after Positivus.

use leptos::prelude::*;

/// Joins class strings, skipping empty ones.
pub fn cn(parts: &[&str]) -> String {
    parts.iter().filter(|p| !p.is_empty()).copied().collect::<Vec<_>>().join(" ")
}

// ---------------------------------------------------------------- Button

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Variant {
    /// Dark ink button (Positivus primary).
    #[default]
    Default,
    /// Lime with offset shadow.
    Brand,
    Outline,
    Secondary,
    Ghost,
    Destructive,
    Link,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Size {
    Sm,
    #[default]
    Md,
    Lg,
    Icon,
}

pub fn button_class(variant: Variant, size: Size) -> String {
    let base = "inline-flex shrink-0 items-center justify-center gap-2 whitespace-nowrap rounded-lg font-medium \
                cursor-pointer transition-[color,background-color,box-shadow,transform] outline-none \
                focus-visible:ring-4 focus-visible:ring-ring/70 disabled:pointer-events-none disabled:opacity-50";
    let v = match variant {
        Variant::Default => "bg-primary text-primary-foreground hover:bg-primary/85",
        Variant::Brand => {
            "bg-accent text-accent-foreground border border-border shadow-brut-sm \
             hover:-translate-y-0.5 hover:shadow-brut active:translate-y-0.5 active:shadow-none"
        }
        Variant::Outline => "border border-border bg-background hover:bg-accent",
        Variant::Secondary => "bg-secondary text-secondary-foreground hover:bg-secondary/70",
        Variant::Ghost => "hover:bg-muted",
        Variant::Destructive => "bg-destructive text-destructive-foreground hover:bg-destructive/90",
        Variant::Link => "underline-offset-4 hover:underline",
    };
    let s = match (variant, size) {
        (Variant::Link, _) => "h-auto px-0 py-0",
        (_, Size::Sm) => "h-9 px-3 text-sm",
        (_, Size::Md) => "h-11 px-5",
        (_, Size::Lg) => "h-14 px-8 text-lg",
        (_, Size::Icon) => "size-10",
    };
    cn(&[base, v, s])
}

#[component]
pub fn Button(
    #[prop(optional)] variant: Variant,
    #[prop(optional)] size: Size,
    /// `button` (default) or `submit`.
    #[prop(default = "button")]
    kind: &'static str,
    #[prop(optional, into)] disabled: MaybeProp<bool>,
    #[prop(optional, into)] class: String,
    children: Children,
) -> impl IntoView {
    view! {
        <button
            type=kind
            class=cn(&[&button_class(variant, size), &class])
            disabled=move || disabled.get().unwrap_or(false)
        >
            {children()}
        </button>
    }
}

/// A router link styled as a button.
#[component]
pub fn ButtonLink(
    #[prop(into)] href: String,
    #[prop(optional)] variant: Variant,
    #[prop(optional)] size: Size,
    #[prop(optional, into)] class: String,
    children: Children,
) -> impl IntoView {
    view! {
        <leptos_router::components::A href=href attr:class=cn(&[&button_class(variant, size), &class])>
            {children()}
        </leptos_router::components::A>
    }
}

// ---------------------------------------------------------------- Card

/// Positivus card colours.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tone {
    #[default]
    Plain,
    Muted,
    Brand,
    Dark,
}

impl Tone {
    pub fn class(self) -> &'static str {
        match self {
            Tone::Plain => "bg-card text-card-foreground",
            Tone::Muted => "bg-muted text-foreground",
            Tone::Brand => "bg-accent text-accent-foreground",
            Tone::Dark => "bg-primary text-primary-foreground",
        }
    }
}

#[component]
pub fn Card(
    #[prop(optional)] tone: Tone,
    /// Large 45px Positivus radius (marketing pages) instead of 24px.
    #[prop(optional)]
    large: bool,
    #[prop(optional, into)] class: String,
    children: Children,
) -> impl IntoView {
    let radius = if large { "rounded-card" } else { "rounded-3xl" };
    view! {
        <div class=cn(&["border border-border shadow-brut", radius, tone.class(), &class])>{children()}</div>
    }
}

#[component]
pub fn CardHeader(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
    view! { <div class=cn(&["flex flex-col gap-1.5 p-6", &class])>{children()}</div> }
}

#[component]
pub fn CardTitle(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
    view! { <h3 class=cn(&["text-xl font-medium leading-tight", &class])>{children()}</h3> }
}

#[component]
pub fn CardDescription(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
    view! { <p class=cn(&["text-sm opacity-75", &class])>{children()}</p> }
}

#[component]
pub fn CardContent(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
    view! { <div class=cn(&["px-6 pb-6", &class])>{children()}</div> }
}

// ---------------------------------------------------------------- Badge / Highlight

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BadgeVariant {
    #[default]
    Default,
    Brand,
    Outline,
    Muted,
    Destructive,
}

#[component]
pub fn Badge(
    #[prop(optional)] variant: BadgeVariant,
    #[prop(optional, into)] class: String,
    children: Children,
) -> impl IntoView {
    let v = match variant {
        BadgeVariant::Default => "border-transparent bg-primary text-primary-foreground",
        BadgeVariant::Brand => "border-border bg-accent text-accent-foreground",
        BadgeVariant::Outline => "border-border",
        BadgeVariant::Muted => "border-transparent bg-muted text-muted-foreground",
        BadgeVariant::Destructive => "border-transparent bg-destructive text-destructive-foreground",
    };
    view! {
        <span class=cn(&[
            "inline-flex items-center gap-1 rounded-md border px-2 py-0.5 text-xs font-medium whitespace-nowrap tabular-nums",
            v,
            &class,
        ])>{children()}</span>
    }
}

/// Positivus heading highlight: text on a lime (or white) block.
#[component]
pub fn Highlight(#[prop(optional)] white: bool, #[prop(optional, into)] class: String, children: Children) -> impl IntoView {
    let bg = if white { "bg-white text-foreground" } else { "bg-accent text-accent-foreground" };
    view! {
        <span class=cn(&["rounded-md px-2 box-decoration-clone leading-snug", bg, &class])>{children()}</span>
    }
}

/// Section title in a lime highlight with a short description beside it.
#[component]
pub fn SectionHeading(
    #[prop(into)] title: String,
    #[prop(optional, into)] description: String,
    #[prop(optional, into)] id: String,
) -> impl IntoView {
    view! {
        <div id=id class="mb-10 flex scroll-mt-24 flex-col gap-4 md:flex-row md:items-center md:gap-10">
            <h2 class="text-3xl font-medium md:text-4xl">
                <Highlight>{title}</Highlight>
            </h2>
            {(!description.is_empty()).then(|| view! { <p class="max-w-xl text-base md:text-lg">{description}</p> })}
        </div>
    }
}

// ---------------------------------------------------------------- Alert

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AlertVariant {
    #[default]
    Success,
    Destructive,
    Info,
}

#[component]
pub fn Alert(#[prop(optional)] variant: AlertVariant, children: Children) -> impl IntoView {
    let v = match variant {
        AlertVariant::Success => "border-border bg-accent/60",
        AlertVariant::Destructive => "border-destructive bg-destructive/10 text-destructive",
        AlertVariant::Info => "border-border bg-muted",
    };
    view! {
        <div role="alert" class=cn(&["mb-4 rounded-lg border px-4 py-3 text-sm font-medium", v])>{children()}</div>
    }
}

// ---------------------------------------------------------------- Table

#[component]
pub fn Table(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
    view! {
        <div class=cn(&["w-full overflow-x-auto rounded-3xl border border-border bg-card shadow-brut", &class])>
            <table class="w-full caption-bottom text-sm">{children()}</table>
        </div>
    }
}

#[component]
pub fn TableHeader(children: Children) -> impl IntoView {
    view! { <thead class="bg-muted [&_tr]:border-b [&_tr]:border-border">{children()}</thead> }
}

#[component]
pub fn TableBody(children: Children) -> impl IntoView {
    view! { <tbody class="[&_tr:last-child]:border-0">{children()}</tbody> }
}

/// Class for `<tr>` rows (plain elements keep `class:` toggles available).
pub const TR: &str = "border-b border-hairline transition-colors hover:bg-muted/60";
/// Class for `<th>` cells.
pub const TH: &str = "h-11 px-4 text-left align-middle text-xs font-medium uppercase tracking-wide text-muted-foreground whitespace-nowrap";
/// Class for `<td>` cells.
pub const TD: &str = "px-4 py-3 align-middle";

// ---------------------------------------------------------------- Form controls

pub const INPUT: &str = "flex h-11 w-full min-w-0 rounded-lg border border-input bg-background px-4 text-base \
                         placeholder:text-muted-foreground outline-none transition-shadow \
                         focus-visible:ring-4 focus-visible:ring-ring/70 disabled:opacity-50";
pub const INPUT_SM: &str = "flex h-9 min-w-0 rounded-md border border-input bg-background px-3 text-sm \
                            outline-none focus-visible:ring-4 focus-visible:ring-ring/70";
pub const LABEL: &str = "flex flex-col gap-1.5 text-sm font-medium";

// ---------------------------------------------------------------- Icons (inline SVG, currentColor)

#[component]
pub fn ArrowUpRight(#[prop(optional, into)] class: String) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"
            stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="M7 17 17 7M8 7h9v9"/>
        </svg>
    }
}

/// Positivus "Learn more" control: a dark circle with a lime arrow.
#[component]
pub fn LearnMore(#[prop(optional)] on_dark: bool, #[prop(into)] label: String) -> impl IntoView {
    let circle = if on_dark { "bg-white text-foreground" } else { "bg-primary text-accent" };
    view! {
        <span class="inline-flex items-center gap-3 text-base">
            <span class=cn(&["grid size-10 place-items-center rounded-full", circle])>
                <ArrowUpRight class="size-5"/>
            </span>
            {label}
        </span>
    }
}

#[component]
pub fn Logo(#[prop(optional)] light: bool) -> impl IntoView {
    let fg = if light { "text-white" } else { "text-foreground" };
    view! {
        <svg class=cn(&["size-8", fg]) viewBox="0 0 32 32" aria-hidden="true">
            <path d="M4 7.5c4-1.8 8-1.8 12 0v19c-4-1.8-8-1.8-12 0z" fill="currentColor"/>
            <path d="M28 7.5c-4-1.8-8-1.8-12 0v19c4-1.8 8-1.8 12 0z" fill="#B9FF66" stroke="currentColor" stroke-width="2"/>
        </svg>
    }
}
