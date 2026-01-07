use crate::model::AppConfig;

pub fn load_config() -> AppConfig {
    let content = std::fs::read_to_string("config.yml")
        .expect("Failed to read config.yml");
    serde_yaml::from_str(&content)
        .expect("Failed to parse config.yml")
}
