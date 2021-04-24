use std::io::Write;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    branch_prefix: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub repos: Vec<RepoConfig>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub branches: Vec<BranchConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BranchConfig {
    pub name: String,
    pub branch_name: String,
    pub repo: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RepoConfig {
    pub path: String,
    pub main_branch: String,
}

impl RepoConfig {
    pub fn short_name(&self) -> &str {
        self.path.rsplit('/').next().unwrap()
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

    pub fn get_branch_config(&self, name: &str) -> Option<&BranchConfig> {
        self.branches.iter().find(|x| x.name == name)
    }

    pub fn get_repo_config(&self, repo_name: &str) -> Option<&RepoConfig> {
        self.repos.iter().find(|x| x.short_name() == repo_name)
    }

    pub fn take_configs(self, branch_name: &str) -> Option<(RepoConfig, BranchConfig)> {
        let branch_config = match self.branches.into_iter().find(|x| x.name == branch_name) {
            Some(b) => b,
            None => return None,
        };
        let repo_config = match self
            .repos
            .into_iter()
            .find(|x| x.short_name() == branch_config.repo)
        {
            Some(r) => r,
            None => return None,
        };
        Some((repo_config, branch_config))
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

pub fn get_current_dir_configs() -> (RepoConfig, BranchConfig) {
    let config = get_config();

    let branch_dir = format!("{}/branches/", root_dir());
    let workdir = std::env::current_dir()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    if !workdir.starts_with(&branch_dir) {
        fail!("must be run inside of a g2 branch!");
    }
    let suffix = &workdir[branch_dir.len()..];
    let branch = match suffix.split('/').find(|s| !s.is_empty()) {
        Some(b) => b,
        None => fail!("must be run inside of a g2 branch!"),
    };

    config.take_configs(branch).unwrap()
}
