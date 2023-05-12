use crate::prelude::*;
use std::path::Path;

use crate::config::CuprateConfig;
use abscissa_core::{config, Command, FrameworkError, Runnable};

/// `generate` subcommand
#[derive(clap::Parser, Command, Debug, Default, Clone)]
pub struct GenerateCmd {
    /// File to place the config, if no file is set then the config is outputted
    /// to the terminal.
    file: Option<String>,
}

impl Runnable for GenerateCmd {
    /// Start the application.
    fn run(&self) {
        let default_config = CuprateConfig::default();
        let conf = toml::to_string_pretty(&toml::Value::try_from(default_config).unwrap()).unwrap();
        match self.file {
            Some(ref output_file) => {
                use std::{fs::File, io::Write};
                File::create(output_file)
                    .expect("must be able to open output file")
                    .write_all(conf.as_bytes())
                    .expect("must be able to write output");
            }
            None => println!("{conf:}"),
        }
    }
}

impl config::Override<CuprateConfig> for GenerateCmd {
    // Process the given command line options, overriding settings from
    // a configuration file using explicit flags taken from command-line
    // arguments.
    fn override_config(&self, config: CuprateConfig) -> Result<CuprateConfig, FrameworkError> {
        Ok(config)
    }
}
