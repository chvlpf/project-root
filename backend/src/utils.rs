use std::fs;

pub fn load_model_fields() -> Vec<String> {
    let json_str = fs::read_to_string("src/data/reviews.json").expect("failed to read reviews.json");
    serde_json::from_str::<Vec<String>>(&json_str).expect("invalid reviews.json format")
}
