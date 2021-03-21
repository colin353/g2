use std::io::Write;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    branch_prefix: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    repos: Vec<RepoConfig>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    branches: Vec<BranchConfig>,
}

#[derive(Serialize, Deserialize)]
pub struct BranchConfig {
    pub name: String,
    pub branch_name: String,
    pub repo: String,
}

#[derive(Serialize, Deserialize)]
pub struct RepoConfig {
    pub path: String,
    pub main_branch: String,
}

impl RepoConfig {
    fn short_name(&self) -> &str {
        self.path.rsplit("/").next().unwrap()
    }
}

impl Config {
    fn default() -> Self {
        Config {
            branches: Vec::new(),
            repos: Vec::new(),
            branch_prefix: String::new(),
        }
    }

    pub fn get_repo_config(&self, repo_name: &str) -> Option<&RepoConfig> {
        self.repos.iter().find(|x| x.short_name() == repo_name)
    }

    pub fn add_branch(&mut self, name: String, repo: String) -> String {
        let branch_name = format!("{}{}", self.branch_prefix, name);

        self.branches.retain(|b| b.name != name);
        self.branches.push(BranchConfig {
            name,
            repo,
            branch_name: branch_name.clone(),
        });
        branch_name
    }

    pub fn add_repo(&mut self, path: String, main_branch: String) {
        self.repos.retain(|s| s.path != path);
        self.repos.push(RepoConfig { path, main_branch })
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
