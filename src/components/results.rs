use crate::models::DietAnalysis;
use dioxus::prelude::*;

fn tag_color(tag: &str) -> &'static str {
    let lower = tag.to_lowercase();
    if lower.contains("vegan") || lower.contains("vegetarian") || lower.contains("free") {
        "tag-green"
    } else if lower.contains("contains") {
        "tag-yellow"
    } else if lower.contains("halal") || lower.contains("kosher") {
        "tag-blue"
    } else {
        "tag-gray"
    }
}

fn confidence_class(confidence: &str) -> &'static str {
    match confidence.to_lowercase().as_str() {
        "high" => "confidence-high",
        "medium" => "confidence-medium",
        _ => "confidence-low",
    }
}

#[component]
pub fn Results(analysis: DietAnalysis) -> Element {
    rsx! {
        div { class: "card",
            div { class: "result-header",
                div { class: "food-name", "{analysis.food_name}" }
                span {
                    class: "confidence-badge {confidence_class(&analysis.confidence)}",
                    "{analysis.confidence}"
                }
            }

            p { class: "description", "{analysis.description}" }

            // Dietary Tags
            if !analysis.dietary_tags.is_empty() {
                div {
                    div { class: "section-label", "Dietary Info" }
                    div { class: "tags-container",
                        for tag in &analysis.dietary_tags {
                            span {
                                class: "tag {tag_color(tag)}",
                                key: "{tag}",
                                "{tag}"
                            }
                        }
                    }
                }
            }

            // Allergens
            if !analysis.allergens.is_empty() {
                div { class: "allergen-warning",
                    div { class: "allergen-title", "Allergen Warning" }
                    div { class: "allergen-list",
                        "{analysis.allergens.join(\", \")}"
                    }
                }
            }

            // Macros
            div {
                div { class: "section-label", "Estimated Macros" }
                div { class: "macros-grid",
                    div { class: "macro-item",
                        div { class: "macro-value", "{analysis.macros_estimate.calories}" }
                        div { class: "macro-label", "Calories" }
                    }
                    div { class: "macro-item",
                        div { class: "macro-value", "{analysis.macros_estimate.protein}" }
                        div { class: "macro-label", "Protein" }
                    }
                    div { class: "macro-item",
                        div { class: "macro-value", "{analysis.macros_estimate.carbs}" }
                        div { class: "macro-label", "Carbs" }
                    }
                    div { class: "macro-item",
                        div { class: "macro-value", "{analysis.macros_estimate.fat}" }
                        div { class: "macro-label", "Fat" }
                    }
                    div { class: "macro-item",
                        div { class: "macro-value", "{analysis.macros_estimate.fiber}" }
                        div { class: "macro-label", "Fiber" }
                    }
                }
            }

            // Ingredients
            if !analysis.ingredients_detected.is_empty() {
                div {
                    div { class: "section-label", "Detected Ingredients" }
                    div { class: "ingredients-list",
                        for ingredient in &analysis.ingredients_detected {
                            span {
                                class: "ingredient",
                                key: "{ingredient}",
                                "{ingredient}"
                            }
                        }
                    }
                }
            }

            // Notes
            if !analysis.notes.is_empty() {
                div {
                    div { class: "section-label", "Notes" }
                    div { class: "notes", "{analysis.notes}" }
                }
            }
        }
    }
}
