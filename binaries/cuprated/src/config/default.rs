use std::{
    io::Write,
    path::{Path, PathBuf},
    str::from_utf8,
};

use cuprate_helper::fs::{
    CUPRATE_BLOCKCHAIN_DIR, CUPRATE_CACHE_DIR, CUPRATE_TXPOOL_DIR, DEFAULT_CONFIG_FILE_NAME,
};

/// Creates a config file which will be named [`DEFAULT_CONFIG_FILE_NAME`] in the directory given in [`Path`].
///
/// This will always terminate the program, on success and failure.
pub fn create_default_config_file(path: &Path) -> ! {
    let config_file = path.join(DEFAULT_CONFIG_FILE_NAME);

    tracing::info!("Attempting to create new config file here: {config_file:?}");

    let mut file = match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&config_file)
    {
        Ok(file) => file,
        Err(e) => {
            tracing::error!("Failed to create config file, got error: {e}");
            std::process::exit(1);
        }
    };

    let config = generate_config_text();
    file.write_all(config.as_bytes()).unwrap();

    std::process::exit(0);
}

/// Generates the text of the default config file.
fn generate_config_text() -> String {
    let toml_value_str = |t: &PathBuf| {
        let mut value = String::new();

        serde::Serialize::serialize(t, toml::ser::ValueSerializer::new(&mut value)).unwrap();

        value
    };

    format!(
        include_str!("Cuprate.toml"),
        cache = toml_value_str(&CUPRATE_CACHE_DIR),
        txpool = toml_value_str(&CUPRATE_TXPOOL_DIR),
        blockchain = toml_value_str(&CUPRATE_BLOCKCHAIN_DIR)
    )
}

#[cfg(test)]
mod tests {
    use crate::config::{default::generate_config_text, Config};

    #[test]
    fn generate_config_text_covers_all_values() {
        let text = generate_config_text();
        let table: toml::Table = toml::from_str(&text).unwrap();

        let full_config = Config::default();
        let full_config_table: toml::Table =
            toml::from_str(&toml::to_string(&full_config).unwrap()).unwrap();

        assert_eq!(full_config_table, table);
    }

    #[test]
    fn generate_config_text_is_valid() {
        let text = generate_config_text();

        let config: Config = toml::from_str(&text).unwrap();
    }
}
