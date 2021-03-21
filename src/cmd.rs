#[macro_use]
use crate::fail;
use crate::conf;

use std::process::{Command, Stdio};

use git2;

fn run_passthrough(mut c: Command) {
    c.stdout(Stdio::inherit());
    c.stderr(Stdio::inherit());
    c.stdin(Stdio::inherit());
    c.output();
}

fn get_stdout(mut c: Command) -> String {
    match c.output() {
        Ok(result) => {
            if !result.status.success() {
                return std::str::from_utf8(&result.stderr)
                    .unwrap()
                    .trim()
                    .to_owned();
            }

            return std::str::from_utf8(&result.stdout)
                .unwrap()
                .trim()
                .to_owned();
        }
        Err(e) => {
            fail!("command failed! {:?}", e)
        }
    }
}

pub fn clone(repo_path: &str) {
    let root_dir = conf::root_dir();
    let repo_name: &str = repo_path.rsplit("/").next().unwrap();
    let destination = format!("{}/repos/{}", root_dir, repo_name);
    if std::path::Path::new(&destination).exists() {
        fail!("repository {:?} is already checked out!", destination);
    }

    std::fs::create_dir_all(format!("{}/repos/", root_dir)).unwrap();

    let mut c = std::process::Command::new("git");
    c.current_dir(format!("{}/repos/", root_dir));
    c.arg("clone");
    c.arg("--bare");
    c.arg(repo_path);
    run_passthrough(c);

    println!("Checked out {} to {}", repo_path, destination);
}

pub fn branch(branch_name: &str) {
    let root_dir = conf::root_dir();
    std::fs::create_dir_all(format!("{}/branches/", root_dir)).unwrap();
    let destination = format!("{}/branches/{}", root_dir, branch_name);

    if std::path::Path::new(&destination).exists() {
        // The branch already exists, just go there
        println!("go to {}", format!("{}/fs/{}/", root_dir, branch_name));
    }
}
