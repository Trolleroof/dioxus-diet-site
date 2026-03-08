use dioxus::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use js_sys::Reflect;

#[component]
pub fn QueryInput(
    query: Signal<String>,
    is_recording: Signal<bool>,
    can_send: bool,
    on_submit: EventHandler<()>,
    on_clear: EventHandler<()>,
) -> Element {
    let mut recognition: Signal<Option<JsValue>> = use_signal(|| None);

    // Initialize speech recognition
    use_effect(move || {
        if let Some(window) = web_sys::window() {
            let sr_class = window
                .get("webkitSpeechRecognition")
                .or_else(|| window.get("SpeechRecognition"));

            if let Some(class) = sr_class {
                if let Ok(constructor) = class.dyn_into::<js_sys::Function>() {
                    let args = js_sys::Array::new();
                    if let Ok(instance) = Reflect::construct(&constructor, &args) {
                        recognition.set(Some(instance));
                    }
                }
            }
        }
    });

    // Single toggle closure avoids mismatched closure types
    let toggle_recording = move |_| {
        if is_recording() {
            // Stop
            if let Some(rec) = recognition() {
                if let Ok(stop) = Reflect::get(&rec, &JsValue::from_str("stop")) {
                    if let Ok(stop_fn) = stop.dyn_into::<js_sys::Function>() {
                        let _ = stop_fn.call0(&rec);
                    }
                }
            }
            is_recording.set(false);
        } else {
            // Start
            if let Some(rec) = recognition() {
                let mut query = query;
                let mut is_recording = is_recording;

                let onresult = Closure::wrap(Box::new(move |e: JsValue| {
                    let results = Reflect::get(&e, &JsValue::from_str("results"))
                        .unwrap_or(JsValue::NULL);
                    let results_arr = js_sys::Array::from(&results);
                    let result = results_arr.get(0);
                    let result_arr = js_sys::Array::from(&result);
                    let item = result_arr.get(0);
                    if let Ok(transcript) = Reflect::get(&item, &JsValue::from_str("transcript")) {
                        if let Some(text) = transcript.as_string() {
                            query.set(text);
                        }
                    }
                    is_recording.set(false);
                }) as Box<dyn FnMut(JsValue)>);

                let onerror = Closure::wrap(Box::new(move |_: JsValue| {
                    is_recording.set(false);
                }) as Box<dyn FnMut(JsValue)>);

                let onend = Closure::wrap(Box::new(move |_: JsValue| {
                    is_recording.set(false);
                }) as Box<dyn FnMut(JsValue)>);

                let _ = Reflect::set(&rec, &JsValue::from_str("onresult"), onresult.as_ref());
                let _ = Reflect::set(&rec, &JsValue::from_str("onerror"), onerror.as_ref());
                let _ = Reflect::set(&rec, &JsValue::from_str("onend"), onend.as_ref());

                onresult.forget();
                onerror.forget();
                onend.forget();

                if let Ok(start) = Reflect::get(&rec, &JsValue::from_str("start")) {
                    if let Ok(start_fn) = start.dyn_into::<js_sys::Function>() {
                        let _ = start_fn.call0(&rec);
                        is_recording.set(true);
                    }
                }
            } else if let Some(window) = web_sys::window() {
                let _ = window.alert_with_message("Speech recognition not available. Please type your question.");
            }
        }
    };

    rsx! {
        div { class: "card",
            div { class: "composer-shell",
                div { class: "composer-meta",
                    div { class: "composer-label", "Ask about the uploaded food photo" }
                    button {
                        class: "btn btn-ghost btn-small",
                        onclick: move |_| on_clear.call(()),
                        "Clear"
                    }
                }
                div { class: "composer-row",
                    textarea {
                        class: "text-input composer-input",
                        placeholder: "Ask about the food in your photo...",
                        value: "{query}",
                        rows: "1",
                        oninput: move |e| {
                            query.set(e.value());
                        },
                        onkeydown: move |e| {
                            if e.key() == Key::Enter && !e.modifiers().shift() {
                                e.prevent_default();
                                if can_send {
                                    on_submit.call(());
                                }
                            }
                        },
                    }
                    div { class: "composer-actions",
                        button {
                            class: if is_recording() { "btn btn-red composer-btn composer-mic-btn" } else { "btn btn-outline composer-btn composer-mic-btn" },
                            onclick: toggle_recording,
                            title: if is_recording() { "Stop voice input" } else { "Start voice input" },
                            svg {
                                class: "icon icon-mic",
                                view_box: "0 0 24 24",
                                fill: "none",
                                stroke: "currentColor",
                                stroke_width: "1.8",
                                stroke_linecap: "round",
                                stroke_linejoin: "round",
                                path { d: "M12 3.75a3.25 3.25 0 0 1 3.25 3.25v5a3.25 3.25 0 1 1-6.5 0V7A3.25 3.25 0 0 1 12 3.75Z" }
                                path { d: "M6.75 11.25v.75a5.25 5.25 0 1 0 10.5 0v-.75" }
                                path { d: "M12 17.25v3" }
                                path { d: "M9.5 20.25h5" }
                            }
                        }
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
            if is_recording() {
                div {
                    class: "recording-indicator",
                    style: "margin-top: 8px; color: #ef4444; font-size: 0.875rem;",
                    "Listening..."
                }
            }
        }
    }
}
