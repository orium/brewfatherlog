use std::path::Path;

#[derive(serde_derive::Deserialize, Debug)]
pub struct GrainfatherAuth {
    pub email: String,
    pub password: String,
}

#[derive(serde_derive::Deserialize, Debug)]
pub struct Grainfather {
    pub auth: GrainfatherAuth,
}

#[derive(serde_derive::Deserialize, Debug)]
pub struct Brewfather {
    pub logging_id: String,
}

#[derive(serde_derive::Deserialize, Debug)]
pub struct Config {
    pub grainfather: Grainfather,
    pub brewfather: Brewfather,
}

impl Config {
    pub fn from_config_file(config_file_path: &Path) -> Config {
        let config_string =
            std::fs::read_to_string(config_file_path).expect("failed to read configuration file");

        Config::from_toml(&config_string)
    }

    fn from_toml(string: &str) -> Config {
        toml::from_str::<Config>(string).expect("failed to parse default configuration file")
    }

    pub fn create_file_if_nonexistent(config_file_path: &Path) -> bool {
        match config_file_path.exists() {
            false => {
                let default_config: &str = include_str!("default-config.toml");

                std::fs::create_dir_all(config_file_path.parent().unwrap())
                    .expect("Failed to create the program directory");

                std::fs::write(config_file_path, default_config)
                    .expect("failed to write default configuration file");

                true
            }
            true => false,
        }
    }
}
