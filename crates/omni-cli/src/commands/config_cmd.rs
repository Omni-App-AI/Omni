use std::path::Path;

use omni_core::config::OmniConfig;

use crate::ConfigAction;

pub fn run(action: ConfigAction, config: &OmniConfig, config_path: &Path) -> anyhow::Result<()> {
    match action {
        ConfigAction::Show => {
            let toml_str = toml::to_string_pretty(config)?;
            println!("{}", toml_str);
        }
        ConfigAction::Validate => {
            let issues = config.validate();
            if issues.is_empty() {
                println!("Config is valid.");
            } else {
                println!("Config issues found:");
                for issue in &issues {
                    println!("  - {}", issue);
                }
                std::process::exit(1);
            }
        }
        ConfigAction::Edit => {
            let editor = std::env::var("EDITOR").unwrap_or_else(|_| {
                if cfg!(target_os = "windows") {
                    "notepad".to_string()
                } else {
                    "nano".to_string()
                }
            });

            // Ensure config file exists with defaults
            if !config_path.exists() {
                OmniConfig::generate_default_file(config_path)?;
                println!("Generated default config at {}", config_path.display());
            }

            let status = std::process::Command::new(&editor)
                .arg(config_path)
                .status()?;

            if !status.success() {
                anyhow::bail!("Editor exited with non-zero status");
            }
        }
        ConfigAction::Reset => {
            print!("Reset config to defaults? This will overwrite {}. [y/N] ", config_path.display());
            use std::io::Write;
            std::io::stdout().flush()?;

            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            if input.trim().eq_ignore_ascii_case("y") {
                OmniConfig::generate_default_file(config_path)?;
                println!("Config reset to defaults.");
            } else {
                println!("Cancelled.");
            }
        }
    }
    Ok(())
}
