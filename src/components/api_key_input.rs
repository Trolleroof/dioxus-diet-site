use dioxus::prelude::*;

const STORAGE_KEY: &str = "diet_scanner_api_key";

fn get_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

fn load_key() -> String {
    get_storage()
        .and_then(|s| s.get_item(STORAGE_KEY).ok())
        .flatten()
        .unwrap_or_default()
}

fn save_key(key: &str) {
    if let Some(storage) = get_storage() {
        let _ = storage.set_item(STORAGE_KEY, key);
    }
}

fn clear_key() {
    if let Some(storage) = get_storage() {
        let _ = storage.remove_item(STORAGE_KEY);
    }
}

#[component]
pub fn ApiKeyInput(api_key: Signal<String>) -> Element {
    let mut input_value = use_signal(|| load_key());
    let is_saved = use_memo(move || !api_key().is_empty());

    // Load saved key on mount
    use_effect(move || {
        let saved = load_key();
        if !saved.is_empty() {
            api_key.set(saved.clone());
            input_value.set(saved);
        }
    });

    rsx! {
        div { class: "card",
            div { class: "card-title", "OpenRouter API Key" }
            if is_saved() {
                div {
                    div { class: "saved-badge", "Key saved" }
                    button {
                        class: "btn btn-outline btn-small",
                        style: "margin-top: 8px;",
                        onclick: move |_| {
                            clear_key();
                            api_key.set(String::new());
                            input_value.set(String::new());
                        },
                        "Clear Key"
                    }
                }
            } else {
                div { class: "input-group",
                    input {
                        class: "text-input",
                        r#type: "password",
                        placeholder: "sk-or-...",
                        value: "{input_value}",
                        oninput: move |e| {
                            input_value.set(e.value());
                        },
                    }
                    button {
                        class: "btn btn-green btn-small",
                        disabled: input_value().trim().is_empty(),
                        onclick: move |_| {
                            let key = input_value().trim().to_string();
                            save_key(&key);
                            api_key.set(key);
                        },
                        "Save"
                    }
                }
            }
        }
    }
}
