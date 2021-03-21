use crate::conf;

use std::process::{Command, Stdio};

use git2;

fn run_passthrough(mut c: Command) {
    c.stdout(Stdio::inherit());
    c.stderr(Stdio::inherit());
    c.stdin(Stdio::inherit());
    c.output().unwrap();
}

fn guess_default_branch(repo: git2::Repository) -> &'static str {
    if let Ok(_) = repo.find_branch("develop", git2::BranchType::Local) {
        return "develop";
    }

    if let Ok(_) = repo.find_branch("main", git2::BranchType::Local) {
        return "main";
    }

    return "master";
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
    c.arg(&repo_path);
    run_passthrough(c);

    let repo = git2::Repository::open_bare(&destination).unwrap();

    let default_branch = guess_default_branch(repo);

    let mut config = conf::get_config();
    config.add_repo(repo_path.to_string(), default_branch.to_string());
    conf::set_config(&config);

    println!("Checked out {} to {}", repo_path, destination);
    println!(
        "Guessed default branch is `{}`, edit ~/.g2/g2.toml if that's not correct.",
        default_branch,
    );
}

pub fn branch_existing(branch_name: &str) {
    let root_dir = conf::root_dir();
    std::fs::create_dir_all(format!("{}/branches/", root_dir)).unwrap();

    let destination = format!("{}/branches/{}", root_dir, branch_name);
    if std::path::Path::new(&destination).exists() {
        // The branch already exists, just go there
        println!("go to {}", format!("{}/fs/{}/", root_dir, branch_name));
    }

    fail!("branch doesn't exist! create it with `g2 branch <repo_name> <branch_name>`");
}

pub fn branch_new(repo_name: &str, branch_name: &str) {
    let root_dir = conf::root_dir();

    let mut config = conf::get_config();

    let repo_config = match config.get_repo_config(repo_name) {
        Some(x) => x,
        None => fail!(
            "repo `{}` isn't cloned! do `g2 clone <repo_path>` first",
            repo_name
        ),
    };

    let repo = git2::Repository::open_bare(format!("{}/repos/{}", root_dir, repo_name)).unwrap();

    let branch = repo
        .find_branch(&repo_config.main_branch, git2::BranchType::Local)
        .unwrap();
    let reference = branch.into_reference();
    let commit = reference.peel_to_commit().unwrap();

    // Check that the branch doesn't already exist
    let full_branch_name = config.add_branch(branch_name.to_string(), repo_name.to_string());
    if let Ok(_) = repo.find_branch(&full_branch_name, git2::BranchType::Local) {
        fail!("branch `{}` already exists!", branch_name);
    }

    repo.branch(&full_branch_name, &commit, false).unwrap();

    std::fs::create_dir_all(format!("{}/branches/{}", root_dir, branch_name)).unwrap();

    conf::set_config(&config);
}

pub fn branch(args: &[String]) {
    match args.len() {
        1 => {
            branch_existing(&args[0]);
        }
        2 => {
            branch_new(&args[0], &args[1]);
        }
        _ => fail!("too many arguments to `branch`!"),
    };
}
