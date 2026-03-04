use omni_core::config::OmniConfig;
use omni_core::database::Database;
use omni_core::paths::OmniPaths;

pub fn run(config: &OmniConfig, paths: &OmniPaths) -> anyhow::Result<()> {
    println!("Omni Status");
    println!("===========");
    println!();
    println!("Config directory: {}", paths.config_dir.display());
    println!("Config file:      {}", paths.config_file().display());
    println!("Data directory:   {}", paths.data_dir.display());
    println!("Database file:    {}", paths.database_file().display());
    println!("Log directory:    {}", paths.log_dir.display());
    println!();

    let db_path = paths.database_file();
    if db_path.exists() {
        let metadata = std::fs::metadata(&db_path)?;
        let size_kb = metadata.len() / 1024;
        println!("Database:         exists ({} KB)", size_kb);

        // Try to open and get session count
        match omni_core::database::get_or_create_encryption_key() {
            Ok(key) => match Database::open(&db_path, &key) {
                Ok(db) => {
                    let count = db.session_count().unwrap_or(0);
                    println!("Sessions:         {}", count);
                }
                Err(e) => println!("Database:         error opening ({})", e),
            },
            Err(_) => println!("Sessions:         (cannot access - no encryption key)"),
        }
    } else {
        println!("Database:         not created yet");
    }

    println!();
    println!("Log level:        {}", config.general.log_level);
    println!("Telemetry:        {}", config.general.telemetry);
    println!("Guardian:         {}", if config.guardian.enabled { "enabled" } else { "disabled" });
    println!("Guardian mode:    {}", config.guardian.sensitivity);
    println!("Default policy:   {}", config.permissions.default_policy);
    println!("Providers:        {}", config.providers.len());

    for (name, provider) in &config.providers {
        println!(
            "  - {} ({}{})",
            name,
            provider.provider_type,
            if provider.enabled { "" } else { ", disabled" }
        );
    }

    Ok(())
}
