use std::process::Stdio;

pub fn teleport(path: &str) {
    std::fs::write("/tmp/g2-destination", path).unwrap();
    std::process::exit(3);
}

pub fn system(
    binary: &str,
    args: &[&str],
    workdir: Option<&str>,
    passthrough: bool,
) -> (String, Result<i32, i32>) {
    let mut c = std::process::Command::new(binary);

    if let Some(d) = workdir {
        c.current_dir(d);
    }
    c.args(args);

    if passthrough {
        c.stdout(Stdio::inherit());
        c.stderr(Stdio::inherit());
        c.stdin(Stdio::inherit());
    }

    let output = match c.output() {
        Ok(x) => x,
        Err(_) => {
            fail!(
                "unable to find `{}`, is it installed and available in your $PATH?",
                binary
            )
        }
    };

    let result =
        String::from_utf8(output.stdout).unwrap() + std::str::from_utf8(&output.stderr).unwrap();
    if output.status.success() {
        (result, Ok(output.status.code().unwrap()))
    } else {
        (result, Err(output.status.code().unwrap()))
    }
}
