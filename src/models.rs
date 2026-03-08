use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DietAnalysis {
    pub food_name: String,
    pub description: String,
    pub dietary_tags: Vec<String>,
    pub allergens: Vec<String>,
    pub macros_estimate: MacrosEstimate,
    pub ingredients_detected: Vec<String>,
    pub confidence: String,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MacrosEstimate {
    pub calories: String,
    pub protein: String,
    pub carbs: String,
    pub fat: String,
    pub fiber: String,
}
