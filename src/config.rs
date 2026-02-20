use serde::Deserialize;
use std::io::Write;
use std::path::PathBuf;

#[derive(Deserialize)]
struct ConfigFile {
    api_key: Option<String>,
    model: Option<String>,
}

pub struct Config {
    pub api_key: String,
    pub model: String,
}

pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("whisper-paste")
        .join("config.toml")
}

pub fn save_api_key(key: &str) -> Result<(), String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("failed to create config dir: {e}"))?;
    }

    // If config exists, update the key in place; otherwise create new
    let content = if path.exists() {
        let existing = std::fs::read_to_string(&path).unwrap_or_default();
        let mut cfg: toml::Table = toml::from_str(&existing).unwrap_or_default();
        cfg.insert("api_key".into(), toml::Value::String(key.to_string()));
        toml::to_string_pretty(&cfg).map_err(|e| e.to_string())?
    } else {
        format!("api_key = \"{key}\"\n")
    };

    std::fs::write(&path, content).map_err(|e| format!("failed to write config: {e}"))?;
    Ok(())
}

pub fn setup_interactive() {
    let path = config_path();
    println!("whisper-paste setup");
    println!("-------------------");
    println!("Config location: {}", path.display());
    println!();

    // Check if key already exists
    if let Ok(existing) = std::fs::read_to_string(&path) {
        if let Ok(cfg) = toml::from_str::<ConfigFile>(&existing) {
            if let Some(ref key) = cfg.api_key {
                if key != "sk-your-key-here" {
                    let masked = format!("{}...{}", &key[..7.min(key.len())], &key[key.len().saturating_sub(4)..]);
                    println!("Existing API key found: {masked}");
                    print!("Replace it? [y/N] ");
                    std::io::stdout().flush().ok();
                    let mut answer = String::new();
                    std::io::stdin().read_line(&mut answer).ok();
                    if !answer.trim().eq_ignore_ascii_case("y") {
                        println!("Keeping existing key.");
                        return;
                    }
                }
            }
        }
    }

    print!("Enter your OpenAI API key: ");
    std::io::stdout().flush().ok();
    let mut key = String::new();
    std::io::stdin().read_line(&mut key).ok();
    let key = key.trim();

    if key.is_empty() {
        eprintln!("No key provided. Aborting.");
        std::process::exit(1);
    }

    match save_api_key(key) {
        Ok(()) => {
            println!("API key saved to {}", path.display());
            println!();
            println!("You're all set! Run `whisper-paste` to start.");
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}

pub fn load_config() -> Config {
    let path = config_path();

    let file_cfg: ConfigFile = if path.exists() {
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        toml::from_str(&content).unwrap_or(ConfigFile {
            api_key: None,
            model: None,
        })
    } else {
        ConfigFile {
            api_key: None,
            model: None,
        }
    };

    let api_key = std::env::var("OPENAI_API_KEY")
        .ok()
        .or(file_cfg.api_key)
        .unwrap_or_else(|| {
            eprintln!("No API key found.");
            eprintln!();
            eprintln!("Run:  whisper-paste --setup");
            eprintln!("  or: whisper-paste --api-key sk-your-key");
            eprintln!("  or: export OPENAI_API_KEY=\"sk-your-key\"");
            std::process::exit(1);
        });

    if api_key == "sk-your-key-here" {
        eprintln!("API key is still the placeholder. Run: whisper-paste --setup");
        std::process::exit(1);
    }

    let model = file_cfg
        .model
        .unwrap_or_else(|| "whisper-1".to_string());

    Config { api_key, model }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_path_is_not_empty() {
        let path = config_path();
        assert!(path.to_str().unwrap().contains("whisper-paste"));
        assert!(path.to_str().unwrap().ends_with("config.toml"));
    }

    #[test]
    fn save_and_load_api_key() {
        let tmp = std::env::temp_dir().join("whisper-paste-test");
        std::fs::create_dir_all(&tmp).unwrap();
        let path = tmp.join("config.toml");

        // Write a config manually
        std::fs::write(&path, "api_key = \"sk-test-12345\"\n").unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let cfg: ConfigFile = toml::from_str(&content).unwrap();
        assert_eq!(cfg.api_key.unwrap(), "sk-test-12345");
        assert!(cfg.model.is_none());

        // Cleanup
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn config_file_with_model() {
        let toml_str = "api_key = \"sk-abc\"\nmodel = \"whisper-1\"\n";
        let cfg: ConfigFile = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.api_key.unwrap(), "sk-abc");
        assert_eq!(cfg.model.unwrap(), "whisper-1");
    }

    #[test]
    fn config_file_empty_is_ok() {
        let cfg: ConfigFile = toml::from_str("").unwrap();
        assert!(cfg.api_key.is_none());
        assert!(cfg.model.is_none());
    }
}
