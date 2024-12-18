use bookrab_core::config::{ensure_config_works, BookrabConfig};

/// Loads the configuration file and makes sure it works.
pub fn ensure_confy_works<'a>() -> BookrabConfig {
    let config: BookrabConfig = confy::load("bookrab", None).unwrap();
    ensure_config_works(&config);
    config
}
