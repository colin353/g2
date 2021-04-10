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

pub fn get_status(mut c: std::process::Command) -> bool {
    match c.output() {
        Ok(result) => return result.status.success(),
        Err(e) => fail!("{:?}", e),
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

pub fn get_files() -> Vec<String> {
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
    output
}

pub fn files() {
    for result in get_files() {
        println!("{}", result);
    }
}

fn snapshot(msg: &str) {
    // Check that there are no SCM change markers in the files to add
    let mut conflicts = Vec::new();
    for file in get_files() {
        if let Ok(s) = std::fs::read_to_string(&file) {
            if s.contains("<<<<<<<") || s.contains(">>>>>>>") {
                conflicts.push(file);
            }
        }
    }

    if !conflicts.is_empty() {
        println!("the following files contain SCM change markers:");
        for conflict in conflicts {
            println!("{}", conflict);
        }
        fail!("resolve the conflicts first, then run `g2 sync` again");
    }

    let mut c = std::process::Command::new("git");
    c.arg("add").arg(".");
    get_stdout(c);

    let mut c = std::process::Command::new("git");
    c.arg("commit").arg("-n").arg("-m").arg(msg);
    get_status(c);
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

    // Try to merge
    let mut c = std::process::Command::new("git");
    c.arg("merge").arg(repo_config.main_branch);
    get_stdout(c);

    // TODO: detect/explain/guide conflict resolution?
}

pub fn upload() {
    println!("try to snapshot");

    let (repo_config, branch_config) = conf::get_current_dir_configs();
    let base = merge_base(&branch_config.branch_name, &repo_config.main_branch);
    snapshot(&branch_config.branch_name);

    println!("try to push");
    let mut c = std::process::Command::new("git");
    c.arg("push")
        .arg("--set-upstream")
        .arg("origin")
        .arg("HEAD");
    get_stdout(c);

    // Check whether a pull request exists
    println!("check for PR");
    let mut c = std::process::Command::new("gh");
    c.arg("pr").arg("view");
    let has_pr = get_status(c);

    if has_pr {
        println!("we got pr");
        return;
    }

    // Create a pull request
    let filename = format!("/tmp/g2.{}.pull-request", branch_config.branch_name);
    let f = std::fs::write(
        &filename,
        "

# Write PR description above. Lines starting with # will be ignored.
",
    )
    .unwrap();

    let editor = match std::env::var("EDITOR") {
        Ok(x) => x,
        Err(_) => String::from("nano"),
    };
    println!("edit description");
    std::process::Command::new(editor)
        .arg(&filename)
        .stdout(Stdio::inherit())
        .stdin(Stdio::inherit())
        .output()
        .unwrap();

    let description = std::fs::read_to_string(&filename).unwrap();
    let mut description_iter = description.lines();
    let mut title = String::new();
    while let Some(line) = description_iter.next() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("#") {
            continue;
        }
        title = line.to_string();
        break;
    }

    if title.is_empty() {
        fail!("PR description was empty, quitting!");
    }

    let body = description_iter
        .filter(|line| !line.trim().starts_with("#"))
        .collect::<Vec<_>>()
        .join("\n");
    let body_filename = format!("/tmp/g2.{}.body", branch_config.branch_name);
    std::fs::write(&body_filename, body).unwrap();

    std::process::Command::new("gh")
        .arg("pr")
        .arg("create")
        .arg("--title")
        .arg(title)
        .arg("--body-file")
        .arg(body_filename)
        .stdout(Stdio::inherit())
        .stdin(Stdio::inherit())
        .output()
        .unwrap();
}
