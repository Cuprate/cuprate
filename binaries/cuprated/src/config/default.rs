use std::{
    io::Write,
    path::{Path, PathBuf},
    str::from_utf8,
};

use cuprate_helper::fs::{CUPRATE_CACHE_DIR, DEFAULT_CONFIG_FILE_NAME};

use crate::constants::EXAMPLE_CONFIG;

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

    let config = EXAMPLE_CONFIG;
    file.write_all(config.as_bytes()).unwrap();

    std::process::exit(0);
}

#[cfg(test)]
mod tests {
    use crate::{config::Config, constants::EXAMPLE_CONFIG};
    #[test]
    fn generate_config_text_is_valid() {
        let config: Config = toml::from_str(EXAMPLE_CONFIG).unwrap();
    }
}
