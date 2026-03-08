use dioxus::prelude::*;

// ── component ─────────────────────────────────────────────────────────────────

#[component]
pub fn QueryInput(
    query: Signal<String>,
    is_recording: Signal<bool>,
    can_send: bool,
    on_submit: EventHandler<()>,
    on_clear: EventHandler<()>,
) -> Element {
    rsx! {
        div { class: "card",
            div { class: "composer-shell",
                div { class: "composer-row",
                    textarea {
                        class: "text-input composer-input",
                        placeholder: "Ask about the food in your photo...",
                        value: "{query}",
                        rows: "1",
                        oninput: move |e| query.set(e.value()),
                        onkeydown: move |e| {
                            if e.key() == Key::Enter && !e.modifiers().shift() {
                                e.prevent_default();
                                if can_send { on_submit.call(()); }
                            }
                        },
                    }
                    div { class: "composer-actions",
                        button {
                            class: "btn btn-green composer-btn composer-send-btn",
                            disabled: !can_send,
                            onclick: move |_| on_submit.call(()),
                            span { class: "send-label", "Send" }
                            svg {
                                class: "icon icon-send",
                                view_box: "0 0 24 24",
                                fill: "none",
                                stroke: "currentColor",
                                stroke_width: "1.9",
                                stroke_linecap: "round",
                                stroke_linejoin: "round",
                                path { d: "M4.75 12h13.5" }
                                path { d: "m12.75 5.5 6.5 6.5-6.5 6.5" }
                            }
                        }
                    }
                }
            }
        }
    }
}
