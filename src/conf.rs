use std::io::Write;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
pub struct Config {
    repos: Vec<RepoConfig>,
    branches: Vec<BranchConfig>,
}

#[derive(Serialize, Deserialize)]
pub struct BranchConfig {
    name: String,
    repo: String,
}

#[derive(Serialize, Deserialize)]
pub struct RepoConfig {
    path: String,
    main_branch: String,
}

impl Config {
    fn default() -> Self {
        Config {
            branches: Vec::new(),
            repos: Vec::new(),
        }
    }
}

pub fn root_dir() -> String {
    format!("{}/.g2", std::env::var("HOME").unwrap())
}

pub fn set_config(config: &Config) {
    let root = root_dir();
    let mut f = std::fs::File::create(format!("{}/g2.toml", root)).unwrap();
    f.write_all(toml::to_string(config).unwrap().as_bytes())
        .unwrap();
}

pub fn get_config() -> Config {
    let root = root_dir();
    let config_path = format!("{}/g2.toml", root);
    let config = if !std::path::Path::new(&config_path).exists() {
        let c = Config::default();
        set_config(&c);
        c
    } else {
        toml::from_str(&std::fs::read_to_string(config_path).unwrap()).unwrap()
    };

    config
}
