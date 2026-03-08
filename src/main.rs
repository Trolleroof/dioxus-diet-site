mod api;
mod components;
mod models;

use components::QueryInput;
use dioxus::prelude::*;
use dioxus::html::HasFileData;
use models::DietAnalysis;
use base64::{engine::general_purpose, Engine as _};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use web_sys::HtmlInputElement;

const MAIN_CSS: Asset = asset!("/assets/main.css");

#[derive(Clone, PartialEq)]
struct ChatMessage {
    role: MessageRole,
    content: String,
}

#[derive(Clone, PartialEq)]
enum MessageRole {
    User,
    Assistant,
    System,
}

const GENERIC_UI_ERROR: &str =
    "Something went wrong while contacting the assistant. Check your server config and try again.";

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn format_inline_markup(input: &str) -> String {
    let escaped = escape_html(input);
    let mut output = String::new();
    let mut rest = escaped.as_str();
    let mut strong = false;

    while let Some(idx) = rest.find("**") {
        output.push_str(&rest[..idx]);
        output.push_str(if strong { "</strong>" } else { "<strong>" });
        strong = !strong;
        rest = &rest[idx + 2..];
    }

    output.push_str(rest);

    if strong {
        output.push_str("</strong>");
    }

    output
}

fn format_message_html(content: &str) -> String {
    let mut html = String::new();
    let mut paragraph_lines: Vec<String> = Vec::new();
    let mut bullet_lines: Vec<String> = Vec::new();

    let flush_paragraph = |html: &mut String, lines: &mut Vec<String>| {
        if !lines.is_empty() {
            let paragraph = lines.join(" ");
            html.push_str("<p>");
            html.push_str(&format_inline_markup(&paragraph));
            html.push_str("</p>");
            lines.clear();
        }
    };

    let flush_bullets = |html: &mut String, bullets: &mut Vec<String>| {
        if !bullets.is_empty() {
            html.push_str("<ul>");
            for bullet in bullets.iter() {
                html.push_str("<li>");
                html.push_str(&format_inline_markup(bullet));
                html.push_str("</li>");
            }
            html.push_str("</ul>");
            bullets.clear();
        }
    };

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            flush_paragraph(&mut html, &mut paragraph_lines);
            flush_bullets(&mut html, &mut bullet_lines);
            continue;
        }

        if let Some(bullet) = trimmed.strip_prefix("* ") {
            flush_paragraph(&mut html, &mut paragraph_lines);
            bullet_lines.push(bullet.to_string());
            continue;
        }

        if !bullet_lines.is_empty() {
            flush_bullets(&mut html, &mut bullet_lines);
        }

        paragraph_lines.push(trimmed.to_string());
    }

    flush_paragraph(&mut html, &mut paragraph_lines);
    flush_bullets(&mut html, &mut bullet_lines);

    if html.is_empty() {
        "<p></p>".to_string()
    } else {
        html
    }
}

fn format_analysis(analysis: &DietAnalysis) -> String {
    let tags = if analysis.dietary_tags.is_empty() {
        "None noted".to_string()
    } else {
        analysis.dietary_tags.join(", ")
    };
    let allergens = if analysis.allergens.is_empty() {
        "None likely visible".to_string()
    } else {
        analysis.allergens.join(", ")
    };
    let ingredients = if analysis.ingredients_detected.is_empty() {
        "Not confidently identified".to_string()
    } else {
        analysis.ingredients_detected.join(", ")
    };
    let notes = if analysis.notes.trim().is_empty() {
        "No extra notes.".to_string()
    } else {
        analysis.notes.clone()
    };

    format!(
        "{name}\n\n{description}\n\nDietary tags: {tags}\nAllergens: {allergens}\nIngredients: {ingredients}\nConfidence: {confidence}\n\nEstimated macros\nCalories: {calories}\nProtein: {protein}\nCarbs: {carbs}\nFat: {fat}\nFiber: {fiber}\n\nNotes: {notes}",
        name = analysis.food_name,
        description = analysis.description,
        tags = tags,
        allergens = allergens,
        ingredients = ingredients,
        confidence = analysis.confidence,
        calories = analysis.macros_estimate.calories,
        protein = analysis.macros_estimate.protein,
        carbs = analysis.macros_estimate.carbs,
        fat = analysis.macros_estimate.fat,
        fiber = analysis.macros_estimate.fiber,
        notes = notes,
    )
}

/// Max dimension (width or height) for resized images sent to the API.
const MAX_IMAGE_DIM: u32 = 1024;

fn load_file_into_chat(
    file: dioxus::html::FileData,
    mut image_base64: Signal<String>,
    mut image_name: Signal<String>,
    mut loading: Signal<bool>,
    mut error: Signal<Option<String>>,
    mut messages: Signal<Vec<ChatMessage>>,
) {
    let file_name = file.name();
    image_name.set(file_name.clone());
    loading.set(true);
    error.set(None);

    spawn(async move {
        match file.read_bytes().await {
            Ok(bytes) => {
                #[cfg(target_arch = "wasm32")]
                {
                    match compress_image_bytes(&bytes).await {
                        Ok(compressed_b64) => {
                            image_base64.set(compressed_b64);
                            messages.with_mut(|items| {
                                items.push(ChatMessage {
                                    role: MessageRole::System,
                                    content: format!("Attached photo: {}", file_name),
                                });
                            });
                        }
                        Err(e) => {
                            let text = format!("Failed to process image: {}", e);
                            error.set(Some(text.clone()));
                            messages.with_mut(|items| {
                                items.push(ChatMessage {
                                    role: MessageRole::System,
                                    content: text,
                                });
                            });
                        }
                    }
                }

                #[cfg(not(target_arch = "wasm32"))]
                {
                    image_base64.set(general_purpose::STANDARD.encode(&bytes));
                    messages.with_mut(|items| {
                        items.push(ChatMessage {
                            role: MessageRole::System,
                            content: format!("Attached photo: {}", file_name),
                        });
                    });
                }
            }
            Err(err) => {
                let text = format!("Could not read the uploaded file: {}", err);
                error.set(Some(text.clone()));
                messages.with_mut(|items| {
                    items.push(ChatMessage {
                        role: MessageRole::System,
                        content: text,
                    });
                });
            }
        }

        loading.set(false);
    });
}

/// Resize the image on the client via an OffscreenCanvas / <canvas> so the
/// base64 payload stays well under the server body-size limit.
#[cfg(target_arch = "wasm32")]
async fn compress_image_bytes(raw: &[u8]) -> Result<String, String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let b64_src = general_purpose::STANDARD.encode(raw);
    let data_url = format!("data:image/jpeg;base64,{}", b64_src);

    let window = web_sys::window().ok_or("no window")?;
    let document = window.document().ok_or("no document")?;

    let img = document
        .create_element("img")
        .map_err(|_| "create_element failed")?
        .dyn_into::<web_sys::HtmlImageElement>()
        .map_err(|_| "dyn_into HtmlImageElement failed")?;

    let (tx, rx) = futures_channel::oneshot::channel::<Result<(), String>>();
    let tx = std::rc::Rc::new(std::cell::RefCell::new(Some(tx)));

    let tx_ok = tx.clone();
    let onload = wasm_bindgen::closure::Closure::wrap(Box::new(move || {
        if let Some(sender) = tx_ok.borrow_mut().take() {
            let _ = sender.send(Ok(()));
        }
    }) as Box<dyn FnMut()>);

    let tx_err = tx.clone();
    let onerror = wasm_bindgen::closure::Closure::wrap(Box::new(move || {
        if let Some(sender) = tx_err.borrow_mut().take() {
            let _ = sender.send(Err("image load error".to_string()));
        }
    }) as Box<dyn FnMut()>);

    img.set_onload(Some(onload.as_ref().unchecked_ref()));
    img.set_onerror(Some(onerror.as_ref().unchecked_ref()));
    img.set_src(&data_url);

    rx.await.map_err(|_| "channel cancelled")??;

    let orig_w = img.natural_width();
    let orig_h = img.natural_height();

    let (w, h) = if orig_w > MAX_IMAGE_DIM || orig_h > MAX_IMAGE_DIM {
        let scale = MAX_IMAGE_DIM as f64 / orig_w.max(orig_h) as f64;
        ((orig_w as f64 * scale) as u32, (orig_h as f64 * scale) as u32)
    } else {
        (orig_w, orig_h)
    };

    let canvas = document
        .create_element("canvas")
        .map_err(|_| "create canvas failed")?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| "dyn_into canvas failed")?;
    canvas.set_width(w);
    canvas.set_height(h);

    let ctx = canvas
        .get_context("2d")
        .map_err(|_| "get_context failed")?
        .ok_or("no 2d context")?
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .map_err(|_| "dyn_into context failed")?;

    ctx.draw_image_with_html_image_element_and_dw_and_dh(&img, 0.0, 0.0, w as f64, h as f64)
        .map_err(|_| "draw_image failed")?;

    let jpeg_data_url = canvas
        .to_data_url_with_type_and_encoder_options("image/jpeg", &wasm_bindgen::JsValue::from_f64(0.75))
        .map_err(|_| "to_data_url failed")?;

    let b64 = jpeg_data_url
        .find(',')
        .map(|i| jpeg_data_url[i + 1..].to_string())
        .ok_or("invalid data url")?;

    Ok(b64)
}

fn main() {
    #[cfg(feature = "server")]
    {
        dotenvy::dotenv().ok();
    }

    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut image_base64 = use_signal(String::new);
    let mut image_name = use_signal(String::new);
    let mut query = use_signal(String::new);
    let mut loading = use_signal(|| false);
    let mut error = use_signal(|| Option::<String>::None);
    let is_recording = use_signal(|| false);
    let mut is_drag_over = use_signal(|| false);
    let mut messages = use_signal(|| {
        vec![ChatMessage {
            role: MessageRole::Assistant,
            content: "Ask about a meal, ingredient, or nutrition question. Attach a photo if you want image-based answers.".to_string(),
        }]
    });

    let can_ask = use_memo(move || {
        !image_base64().is_empty() && !query().trim().is_empty() && !loading()
    });

    let send_message = move |_| {
        let img = image_base64();
        let q = query();
        let trimmed = q.trim().to_string();
        if trimmed.is_empty() {
            return;
        }

        loading.set(true);
        error.set(None);
        query.set(String::new());
        messages.with_mut(|items| {
            items.push(ChatMessage {
                role: MessageRole::User,
                content: trimmed.clone(),
            });
        });

        spawn(async move {
            let response = if img.is_empty() {
                Err(ServerFnError::new("Upload a food photo before sending a message."))
            } else {
                let lower = trimmed.to_lowercase();
                let needs_full_analysis = lower.contains("analy")
                    || lower.contains("scan")
                    || lower.contains("what is this")
                    || lower.contains("summar")
                    || lower.contains("overview");

                if needs_full_analysis {
                    api::analyze_food(img).await.map(|analysis| format_analysis(&analysis))
                } else {
                    api::ask_question(img, trimmed.clone()).await
                }
            };

            match response {
                Ok(result) => {
                    messages.with_mut(|items| {
                        items.push(ChatMessage {
                            role: MessageRole::Assistant,
                            content: result,
                        });
                    });
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    error.set(Some(error_msg.clone()));
                    messages.with_mut(|items| {
                        items.push(ChatMessage {
                            role: MessageRole::System,
                            content: format!("Error: {}", error_msg),
                        });
                    });
                }
            }
            loading.set(false);
        });
    };

    let clear_chat = move |_| {
        image_base64.set(String::new());
        image_name.set(String::new());
        query.set(String::new());
        error.set(None);
        loading.set(false);
        messages.set(vec![ChatMessage {
            role: MessageRole::Assistant,
            content: "Chat cleared. Attach a new food photo or ask another question.".to_string(),
        }]);
    };

    let trigger_file_input = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(window) = web_sys::window() {
                if let Some(document) = window.document() {
                    if let Some(el) = document.get_element_by_id("food-file-input") {
                        if let Ok(input) = el.dyn_into::<HtmlInputElement>() {
                            input.click();
                        }
                    }
                }
            }
        }
    };

    let on_file_change = move |evt: Event<FormData>| {
        if let Some(file) = evt.files().into_iter().next() {
            load_file_into_chat(file, image_base64, image_name, loading, error, messages);
        }
    };

    rsx! {
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        input {
            id: "food-file-input",
            class: "hidden-file-input",
            r#type: "file",
            accept: "image/*",
            capture: "environment",
            // "environment" = rear camera (best for food). Use "user" for front camera.
            onchange: on_file_change,
        }
        div { class: "app-container",
            div { class: "header",
                h1 { "Diet Copilot" }
                p { "One conversation thread for photo-based food questions." }
            }
            div { class: "upload-shell",
                div { class: "upload-shell-header",
                    div {
                        div { class: "eyebrow", "Image Upload" }
                        h2 { class: "upload-title", "Choose the food photo first" }
                    }
                    button {
                        class: "btn btn-outline btn-small",
                        onclick: trigger_file_input,
                        if image_base64().is_empty() { "Upload Photo" } else { "Change Photo" }
                    }
                }
                div {
                    class: if is_drag_over() { "upload-dropzone upload-dropzone-active" } else { "upload-dropzone" },
                    onclick: trigger_file_input,
                    ondragenter: move |evt| {
                        evt.prevent_default();
                        is_drag_over.set(true);
                    },
                    ondragover: move |evt| {
                        evt.prevent_default();
                        is_drag_over.set(true);
                    },
                    ondragleave: move |_| {
                        is_drag_over.set(false);
                    },
                    ondrop: move |evt| {
                        evt.prevent_default();
                        is_drag_over.set(false);
                        if let Some(file) = evt.files().into_iter().next() {
                            load_file_into_chat(file, image_base64, image_name, loading, error, messages);
                        }
                    },
                    div { class: "upload-dropzone-icon",
                        svg {
                            class: "icon icon-upload",
                            view_box: "0 0 24 24",
                            width: "24",
                            height: "24",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "2.2",
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            path { d: "M12 4.75v9" }
                            path { d: "m8.75 8 3.25-3.25L15.25 8" }
                            path { d: "M5 15.75v1.5A1.75 1.75 0 0 0 6.75 19h10.5A1.75 1.75 0 0 0 19 17.25v-1.5" }
                            path { d: "M8 15.75h8" }
                        }
                    }
                    if !image_name().is_empty() {
                        div { class: "upload-dropzone-content",
                            p { class: "upload-dropzone-title", "{image_name}" }
                            p { class: "upload-dropzone-copy", "Drop a replacement here, or tap to choose another photo on iPhone." }
                        }
                        div { class: "toolbar-chip",
                            button {
                                class: "toolbar-chip-clear",
                                onclick: move |evt| {
                                    evt.stop_propagation();
                                    image_base64.set(String::new());
                                    image_name.set(String::new());
                                },
                                "Remove"
                            }
                        }
                    } else {
                        div { class: "upload-dropzone-content",
                            p { class: "upload-dropzone-title", "Drop a food photo here" }
                            p { class: "upload-dropzone-copy", "Drag and drop on desktop, or tap here on iPhone to open the camera or photo library." }
                        }
                    }
                }
            }
            div { class: "chat-shell",
                div { class: "chat-history",
                    for (index, message) in messages().iter().enumerate() {
                        div {
                            key: "{index}",
                            class: match message.role {
                                MessageRole::User => "message message-user",
                                MessageRole::Assistant => "message message-assistant",
                                MessageRole::System => "message message-system",
                            },
                            div { class: "message-role",
                                match message.role {
                                    MessageRole::User => "You",
                                    MessageRole::Assistant => "Assistant",
                                    MessageRole::System => "Status",
                                }
                            }
                            div {
                                class: "message-body",
                                dangerous_inner_html: "{format_message_html(&message.content)}"
                            }
                        }
                    }
                    if loading() {
                        div { class: "message message-assistant",
                            div { class: "message-role", "Assistant" }
                            div { class: "typing",
                                span {}
                                span {}
                                span {}
                            }
                        }
                    }
                }
                QueryInput {
                    query,
                    is_recording,
                    can_send: can_ask(),
                    on_submit: send_message,
                    on_clear: clear_chat,
                }
            }

            if let Some(ref err) = error() {
                div { class: "error-card",
                    p { class: "error-text", "{err}" }
                }
            }


        }
    }
}
