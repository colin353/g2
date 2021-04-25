use crate::{cmd, conf, tui};

fn guess_default_branch(repo: git2::Repository) -> &'static str {
    if repo.find_branch("develop", git2::BranchType::Local).is_ok() {
        return "develop";
    }

    if repo.find_branch("main", git2::BranchType::Local).is_ok() {
        return "main";
    }

    "master"
}

pub fn clone(repo_path: &str) {
    let root_dir = conf::root_dir();
    let repo_name: &str = repo_path.rsplit('/').next().unwrap();
    let destination = format!("{}/repos/{}", root_dir, repo_name);
    if std::path::Path::new(&destination).exists() {
        fail!("repository {:?} is already checked out!", destination);
    }

    std::fs::create_dir_all(format!("{}/repos/", root_dir)).unwrap();

    let (_, result) = cmd::system(
        "git",
        &["clone", "--bare", &repo_path],
        Some(&format!("{}/repos/", root_dir)),
        true,
    );
    if result.is_err() {
        fail!("unable to clone repository!");
    }

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

pub fn get_tmux_name() -> Option<String> {
    let (out, result) = cmd::system("tmux", &["display-message", "-p", "#W"], None, false);
    if result.is_err() {
        return None;
    }
    Some(out.trim().to_string())
}

pub fn set_tmux_name(name: &str) {
    // If tmux is installed, we can set the tmux name. If this command fails, it doesn't matter
    let (_, result) = cmd::system("tmux", &["rename-window", name], None, false);
    if result.is_err() {
        // No need to do anything
    }
}

pub fn branch_existing(branch_name: &str, with_output: bool) {
    let root_dir = conf::root_dir();
    std::fs::create_dir_all(format!("{}/branches/", root_dir)).unwrap();

    let destination = format!("{}/branches/{}", root_dir, branch_name);
    if std::path::Path::new(&destination).exists() {
        // The branch already exists, just go there
        if with_output {
            println!(
                "go to {}",
                format!("{}/branches/{}/", root_dir, branch_name)
            );
        }
        set_tmux_name(branch_name);
        cmd::teleport(&destination);
    }

    if with_output {
        fail!("branch doesn't exist! create it with `g2 branch <repo_name> <branch_name>`");
    }
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

    // Fetch origin. Can't do this with libgit2 because it requires authentication
    let (_, result) = cmd::system(
        "git",
        &[
            "fetch",
            "-q",
            "origin",
            &format!("{}:{}", &repo_config.main_branch, &repo_config.main_branch),
        ],
        Some(&format!("{}/repos/{}", root_dir, repo_name)),
        true,
    );
    if result.is_err() {
        fail!("couldn't fetch origin!");
    }

    let branch = repo.find_branch("main", git2::BranchType::Local).unwrap();
    let reference = branch.into_reference();
    let commit = reference.peel_to_commit().unwrap();

    // Check that the branch doesn't already exist
    let full_branch_name = config.add_branch(branch_name.to_string(), repo_name.to_string());
    if repo
        .find_branch(&full_branch_name, git2::BranchType::Local)
        .is_ok()
    {
        fail!("branch `{}` already exists!", branch_name);
    }

    let branch_ref = repo
        .branch(&full_branch_name, &commit, false)
        .unwrap()
        .into_reference();

    let mut opts = git2::WorktreeAddOptions::new();
    opts.reference(Some(&branch_ref));

    let path = format!("{}/branches/{}", root_dir, branch_name);

    repo.worktree(&full_branch_name, std::path::Path::new(&path), Some(&opts))
        .unwrap();

    conf::set_config(&config);

    println!("created branch {}, now go to `{}`", branch_name, path);
    set_tmux_name(branch_name);
    cmd::teleport(&path);
}

pub fn auto() {
    let name = match get_tmux_name() {
        Some(name) => name,
        // tmux might not be running, quit silently
        None => fail!(),
    };
    branch_existing(&name, false);
}

pub fn new(args: &[String]) {
    if args.is_empty() {
        fail!("you must provide a branch name");
    } else if args.len() == 2 {
        return branch_new(&args[0], &args[1]);
    } else if args.len() != 1 {
        fail!("too many arguments!");
    }

    let config = conf::get_config();
    let options: Vec<_> = config.repos.iter().map(|x| x.short_name()).collect();
    let chosen = match tui::select("choose which repository to use:", &options) {
        Ok(x) => x,
        Err(_) => fail!("you have no repositories! clone one first"),
    };
    branch_new(options[chosen], &args[0]);
}

pub fn branch(args: &[String]) {
    match args.len() {
        0 => {
            // See if we can guess the branch name from tmux
            if let Some(b) = get_tmux_name() {
                let config = conf::get_config();
                if config.get_branch_config(&b).is_some() {
                    return branch_existing(&b, true);
                }
            }

            // Couldn't guess branch name, let's select it via tui
            let config = conf::get_config();
            let options: Vec<_> = config.branches.iter().map(|x| &x.name).collect();

            let chosen = match tui::select("select a branch:", &options) {
                Ok(x) => x,
                Err(_) => fail!("you don't have any branches!"),
            };
            branch_existing(&options[chosen], true);
        }
        1 => {
            branch_existing(&args[0], true);
        }
        2 => {
            branch_new(&args[0], &args[1]);
        }
        _ => fail!("too many arguments to `branch`!"),
    }
}

pub fn get_status(mut c: std::process::Command) -> bool {
    match c.output() {
        Ok(result) => result.status.success(),
        Err(e) => fail!("{:?}", e),
    }
}

pub fn get_stdout(mut c: std::process::Command) -> Result<String, String> {
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
            Ok(output_stdout)
        }
        Err(e) => Err(format!("{:?}", e)),
    }
}

pub fn unwrap_or_fail(input: Result<String, String>) -> String {
    match input {
        Ok(s) => s,
        Err(s) => fail!("{}", s),
    }
}

pub fn merge_base(branch1: &str, branch2: &str) -> String {
    let (out, result) = cmd::system("git", &["merge-base", branch1, branch2], None, false);
    if result.is_err() {
        fail!("failed to read merge base!");
    }
    out.trim().to_owned()
}

pub fn diff() {
    let (repo_config, branch_config) = conf::get_current_dir_configs();
    let base = merge_base(&branch_config.branch_name, &repo_config.main_branch);

    let (_, result) = cmd::system("git", &["diff", &base], None, true);
    if result.is_err() {
        fail!("unable to get diff!");
    }
}

pub fn get_files() -> Vec<String> {
    let (repo_config, branch_config) = conf::get_current_dir_configs();
    let base = merge_base(&branch_config.branch_name, &repo_config.main_branch);

    let (out, result) = cmd::system(
        "git",
        &["--no-pager", "diff", &base, "--name-only"],
        None,
        false,
    );
    if result.is_err() {
        fail!("unable to get diff! error: {}", out);
    }

    let mut output: Vec<_> = out
        .split('\n')
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty())
        .collect();

    let (out, result) = cmd::system(
        "git",
        &["ls-files", "--others", "--exclude-standard"],
        None,
        false,
    );
    if result.is_err() {
        fail!("unable to get diff! error: {}", out);
    }

    for item in out
        .split('\n')
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty())
    {
        output.push(item);
    }

    output.sort();
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
            if s.contains(&format!("<<<{}<<<<", "")) || s.contains(&format!(">>>{}>>>>", "")) {
                conflicts.push(file);
            }
        }
    }

    if !conflicts.is_empty() {
        println!("the following files contain SCM change markers:\n");
        for conflict in conflicts {
            println!("  {}", conflict);
        }
        fail!("\nresolve the conflicts first, then run `g2 sync` again");
    }

    let mut c = std::process::Command::new("git");
    c.arg("add").arg(".");
    unwrap_or_fail(get_stdout(c));

    let mut c = std::process::Command::new("git");
    c.arg("commit").arg("-n").arg("-m").arg(msg);
    get_status(c);
}

pub fn sync() {
    let (repo_config, branch_config) = conf::get_current_dir_configs();

    // Fetch origin. Can't do this with libgit2 because it requires authentication
    let mut c = std::process::Command::new("git");
    c.arg("fetch").arg("-q").arg("origin").arg(format!(
        "{}:{}",
        &repo_config.main_branch, &repo_config.main_branch
    ));
    unwrap_or_fail(get_stdout(c));

    // Snapshot so we can merge incoming changes
    snapshot(&branch_config.branch_name);

    // Try to merge
    let (_, res) = cmd::system("git", &["merge", &repo_config.main_branch], None, false);
    if res.is_err() {
        // There may have been a conflict
        let (out, res) = cmd::system(
            "git",
            &["diff", "--name-only", "--diff-filter=U"],
            None,
            false,
        );
        if res.is_err() {
            fail!("failed to sync!");
        }

        let mut has_conflicts = false;
        for conflict in out.lines().map(|x| x.trim()).filter(|x| !x.is_empty()) {
            if !has_conflicts {
                has_conflicts = true;
                eprintln!("There are some merge conflicts:\n");
            }

            eprintln!(" {}", conflict);
        }

        if !has_conflicts {
            // If there are no conflicts and we failed to sync, then there's a problem
            fail!("unexpectedly failed to sync!");
        }

        eprintln!("\nfix the conflicts, then run `g2 sync` again");
    }
}

pub fn upload() {
    let (_, branch_config) = conf::get_current_dir_configs();
    snapshot(&branch_config.branch_name);

    let (_, result) = cmd::system(
        "git",
        &["push", "--set-upstream", "origin", "HEAD"],
        None,
        true,
    );
    if result.is_err() {
        fail!("failed to push to remote!");
    }

    // Check whether a pull request exists
    let (_, result) = cmd::system("gh", &["pr", "view"], None, false);
    if result.is_ok() {
        return;
    }

    // Create a pull request
    let filename = format!("/tmp/g2.{}.pull-request", branch_config.branch_name);
    std::fs::write(
        &filename,
        "
# Write PR description above. 
# Lines starting with a single # will be ignored.
",
    )
    .unwrap();

    let editor = match std::env::var("EDITOR") {
        Ok(x) => x,
        Err(_) => String::from("nano"),
    };

    let (_, result) = cmd::system(&editor, &[&filename], None, true);
    if result.is_err() {
        fail!("failed to edit PR description, quitting");
    }

    let description = std::fs::read_to_string(&filename).unwrap();
    let mut description_iter = description.lines();
    let mut title = String::new();
    while let Some(line) = description_iter.next() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        title = line.to_string();
        break;
    }

    if title.is_empty() {
        fail!("PR description was empty, quitting!");
    }

    let body = description_iter
        .filter(|line| {
            let trimmed_line = line.trim();
            !trimmed_line.starts_with('#') || trimmed_line.starts_with("##")
        })
        .collect::<Vec<_>>()
        .join("\n");
    let body_filename = format!("/tmp/g2.{}.body", branch_config.branch_name);
    std::fs::write(&body_filename, body).unwrap();

    let (_, result) = cmd::system(
        "gh",
        &[
            "pr",
            "create",
            "--title",
            &title,
            "--body-file",
            &body_filename,
        ],
        None,
        true,
    );
    if result.is_err() {
        eprintln!("failed to create PR!");
    }
}

pub fn clean() {
    let root_dir = conf::root_dir();
    let mut config = conf::get_config();
    config.branches.retain(|branch| {
        let branch_dir = format!("{}/branches/{}", root_dir, branch.name);
        if !std::path::Path::new(&branch_dir).exists() {
            println!("branch {} doesn't exist, cleaning it up", branch.name);
            return false;
        }

        // Check whether a pull request exists
        let (output, result) = cmd::system("gh", &["pr", "view"], Some(&branch_dir), false);
        if result.is_err() {
            // No PR exists yet, keep this branch
            return true;
        }

        if output.contains("MERGED\n") || output.contains("CLOSED\n") {
            println!("branch {} is already merged!", branch.branch_name);

            std::fs::remove_dir_all(&branch_dir).unwrap();
            return false;
        }

        true
    });

    conf::set_config(&config);
}

pub fn status() {
    let (repo_config, branch_config) = conf::get_current_dir_configs();
    let base = merge_base(&branch_config.branch_name, &repo_config.main_branch);

    let mut file_stats = Vec::new();
    for file in get_files() {
        let (out, result) = cmd::system("git", &["diff", "--numstat", &base, &file], None, false);
        if result.is_err() {
            fail!("couldn't run git diff!");
        }
        let numbers: Vec<usize> = out
            .split_whitespace()
            .filter_map(|x| x.parse().ok())
            .collect();
        if numbers.len() != 2 {
            file_stats.push((String::from("[new]"), file));
            continue;
        }

        let num_summary = match (numbers[0], numbers[1]) {
            (0, 0) => String::from("[new]"),
            (x, 0) => format!("[+{}]", x),
            (0, x) => format!("[-{}]", x),
            (x, y) => format!("[+{}, -{}]", x, y),
        };

        file_stats.push((num_summary, file));
    }

    let max_numstats = file_stats.iter().map(|(n, _)| n.len()).max().unwrap_or(0);

    for (num_summary, filename) in file_stats {
        println!("{:>width$} {}", num_summary, filename, width = max_numstats);
    }
}

pub fn revert(args: &[String]) {
    if args.len() != 1 {
        fail!("you must provide exactly one argument, the filename to revert");
    }
    let name = &args[0];

    let (repo_config, branch_config) = conf::get_current_dir_configs();
    let base = merge_base(&branch_config.branch_name, &repo_config.main_branch);

    let (_, res) = cmd::system("git", &["checkout", &base, name], None, false);
    if res.is_err() {
        // Check if the file existed in the version.
        let (_, res) = cmd::system(
            "git",
            &["cat-file", "-e", &format!("{}:{}", base, name)],
            None,
            false,
        );
        if res.is_err() {
            // If the file didn't exist previously, delete it
            match std::fs::remove_file(name) {
                Ok(_) => (),
                Err(_) => {
                    fail!("couldn't revert file! does that file exist?");
                }
            }
        } else {
            fail!("couldn't revert file!");
        }
    }
}

pub fn check() {
    eprintln!("g2 is checking your setup...");

    let mut any_failures = false;

    let (_, result) = cmd::system("which", &["git"], None, false);
    if result.is_err() {
        eprintln!("[err] git isn't installed!");
        eprintln!("To fix this, install git, then try again!\n");
        any_failures = true;
    } else {
        eprintln!(" [ok] the git command exists");
    }

    let (_, result) = cmd::system("which", &["gh"], None, false);
    if result.is_err() {
        eprintln!("[err] the gh command isn't installed!\n");
        eprintln!("To fix this, install the gh command, see https://github.com/cli/cli");
        eprintln!("then try again!\n");
        any_failures = true;
    } else {
        eprintln!(" [ok] the gh command exists");

        // Only check login state if the gh command is installed
        let (_, result) = cmd::system("gh", &["auth", "status"], None, false);
        if result.is_err() {
            eprintln!("[err] you aren't logged into github via gh!\n");

            println!("To fix this, run the command:");
            println!("  gh auth login");
            println!("and then try again!\n");
            any_failures = true;
        } else {
            eprintln!(" [ok] you're logged into github");
        }
    }

    let (_, result) = cmd::system("which", &["tmux"], None, false);
    if result.is_err() {
        eprintln!("[err] tmux isn't installed!\n");
        eprintln!("Installing tmux is optional, but it makes g2 a lot better.");
        eprintln!("Install tmux and try again\n");
    } else {
        eprintln!(" [ok] tmux is installed");

        // Only check if we're in a tmux window if tmux is installed
        if get_tmux_name().is_some() {
            eprintln!(" [ok] you are currently in a tmux window");
        } else {
            eprintln!("[err] you're not in a tmux window!");
        }
    }

    let editor = match std::env::var("SHELL") {
        Ok(x) if x.contains("/zsh") => {
            // Check teleport setup
            let (out, res) = cmd::system("zsh", &["-c", "source ~/.zshrc; type g2"], None, false);
            if out.contains("g2 is a shell function") {
                eprintln!(" [ok] you're using zsh, and teleport is set up correctly");
            } else {
                eprintln!("[err] you're using zsh, but teleport is not set up\n");
                eprintln!("To fix this, add this to your ~/.zshrc:");
                eprintln!(
                    "
    g2 () {{
      G2=`whence -p g2`
      $G2 $@

      if [ $? -eq 3 ]
      then
        cd `cat /tmp/g2-destination`
      fi
    }}
    g2 auto
                    "
                );
            }
        }
        Ok(x) => {
            eprintln!(" [ok] You're using an unsupported shell, so teleport won't work");
        }
        Err(_) => {
            eprintln!("[err] I'm not sure which shell you're using")
        }
    };

    if any_failures {
        fail!();
    }
}
