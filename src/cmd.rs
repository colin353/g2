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

    let branch_ref = repo
        .branch(&full_branch_name, &commit, false)
        .unwrap()
        .into_reference();

    let mut opts = git2::WorktreeAddOptions::new();
    opts.reference(Some(&branch_ref));

    let path = format!("{}/branches/{}", root_dir, branch_name);

    let worktree = repo.worktree(&full_branch_name, std::path::Path::new(&path), Some(&opts));

    conf::set_config(&config);

    println!("created branch {}, now go to `{}`", branch_name, path);
}

pub fn branch(args: &[String]) {
    match args.len() {
        0 => fail!("you must provide a branch name!"),
        1 => {
            branch_existing(&args[0]);
        }
        2 => {
            branch_new(&args[0], &args[1]);
        }
        _ => fail!("too many arguments to `branch`!"),
    };
}

pub fn get_stdout(mut c: std::process::Command) -> String {
    match c.output() {
        Ok(result) => {
            if !result.status.success() {
                let output_stderr = std::str::from_utf8(&result.stderr)
                    .unwrap()
                    .trim()
                    .to_owned();
                fail!("{}", output_stderr);
            }

            let output_stdout = std::str::from_utf8(&result.stdout)
                .unwrap()
                .trim()
                .to_owned();
            output_stdout
        }
        Err(e) => fail!("{:?}", e),
    }
}

pub fn merge_base(branch1: &str, branch2: &str) -> String {
    let mut c = std::process::Command::new("git");
    c.arg("merge-base");
    c.arg(branch1);
    c.arg(branch2);
    get_stdout(c)
}

pub fn diff() {
    let (repo_config, branch_config) = conf::get_current_dir_configs();
    let base = merge_base(&branch_config.branch_name, &repo_config.main_branch);

    let mut c = std::process::Command::new("git");
    c.arg("diff");
    c.arg(base)
        .stdout(std::process::Stdio::inherit())
        .stdin(std::process::Stdio::inherit());

    get_stdout(c);
}

pub fn files() {
    let (repo_config, branch_config) = conf::get_current_dir_configs();
    let base = merge_base(&branch_config.branch_name, &repo_config.main_branch);

    let mut c = std::process::Command::new("git");
    c.arg("--no-pager").arg("diff").arg(base).arg("--name-only");

    let out = get_stdout(c);
    let mut output: Vec<_> = out
        .split("\n")
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty())
        .collect();

    let mut c = std::process::Command::new("git");
    c.arg("ls-files").arg("--others").arg("--exclude-standard");

    let out = get_stdout(c);
    for result in out
        .split("\n")
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty())
    {
        output.push(result);
    }

    for result in output {
        println!("{}", result);
    }
}

fn snapshot(msg: &str) {
    let mut c = std::process::Command::new("git");
    c.arg("add").arg(".");
    get_stdout(c);

    let mut c = std::process::Command::new("git");
    c.arg("commit").arg("-n").arg("-m").arg(msg);
    get_stdout(c);
}

pub fn sync() {
    let (repo_config, branch_config) = conf::get_current_dir_configs();
    let base = merge_base(&branch_config.branch_name, &repo_config.main_branch);

    // Fetch origin
    let mut c = std::process::Command::new("git");
    c.arg("fetch");
    get_stdout(c);

    // Snapshot so we can merge incoming changes
    snapshot(&branch_config.branch_name);
}
