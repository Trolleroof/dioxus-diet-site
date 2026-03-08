use dioxus::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlInputElement, FileReader};

#[component]
pub fn ImageCapture(
    image_base64: Signal<String>,
    image_preview_url: Signal<String>,
) -> Element {
    let mut is_processing = use_signal(|| false);

    let trigger_file_input = move |_| {
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                if let Some(el) = document.get_element_by_id("food-file-input") {
                    if let Ok(input) = el.dyn_into::<HtmlInputElement>() {
                        input.click();
                    }
                }
            }
        }
    };

    let on_file_change = move |_: Event<FormData>| {
        let window = match web_sys::window() { Some(w) => w, None => return };
        let document = match window.document() { Some(d) => d, None => return };
        let el = match document.get_element_by_id("food-file-input") { Some(e) => e, None => return };
        let input: HtmlInputElement = match el.dyn_into() { Ok(i) => i, Err(_) => return };
        let files = match input.files() { Some(f) => f, None => return };
        let file = match files.get(0) { Some(f) => f, None => return };

        is_processing.set(true);

        let reader = match FileReader::new() { Ok(r) => r, Err(_) => { is_processing.set(false); return; } };
        let reader_clone = reader.clone();

        let onload = {
            let mut image_base64 = image_base64;
            let mut image_preview_url = image_preview_url;
            let mut is_processing = is_processing;

            Closure::wrap(Box::new(move |_: web_sys::Event| {
                if let Ok(result) = reader_clone.result() {
                    let data_url = result.as_string().unwrap_or_default();
                    image_preview_url.set(data_url.clone());
                    if let Some(idx) = data_url.find(',') {
                        image_base64.set(data_url[idx + 1..].to_string());
                    }
                }
                is_processing.set(false);
            }) as Box<dyn FnMut(_)>)
        };

        reader.set_onload(Some(onload.as_ref().unchecked_ref()));
        onload.forget();
        let _ = reader.read_as_data_url(&file);
    };

    rsx! {
        div { class: "card",
            div { class: "card-title", "Food Image" }

            // Completely hidden native input
            input {
                id: "food-file-input",
                style: "position:absolute;width:1px;height:1px;opacity:0;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap",
                r#type: "file",
                accept: "image/*",
                capture: "environment",
                onchange: on_file_change,
            }

            if !image_preview_url().is_empty() {
                div { class: "preview-container",
                    img {
                        class: "preview-image",
                        src: "{image_preview_url}",
                        alt: "Food preview",
                    }
                    div { class: "preview-actions",
                        button {
                            class: "btn btn-outline",
                            onclick: trigger_file_input,
                            "Change"
                        }
                        button {
                            class: "btn btn-outline",
                            onclick: move |_| {
                                image_base64.set(String::new());
                                image_preview_url.set(String::new());
                            },
                            "Remove"
                        }
                    }
                }
            } else if is_processing() {
                div { class: "capture-area",
                    div { class: "loading-container",
                        div { class: "spinner" }
                        div { class: "loading-text", "Processing image..." }
                    }
                }
            } else {
                div {
                    class: "capture-area",
                    onclick: trigger_file_input,
                    div { class: "capture-icon", "📸" }
                    div { class: "capture-text", "Tap to take a photo or upload" }
                    div { class: "capture-subtext", "Point your camera at any food item" }
                }
            }
        }
    }
}
