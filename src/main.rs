#[macro_use]
mod fail;

mod actions;
mod cmd;
mod conf;
mod tui;

fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 2 {
        fail!("you need to provide at least one argument!");
    }
    match args[1].as_str() {
        "clone" => actions::clone(args[2].as_str()),
        "branch" => actions::branch(&args[2..]),
        "diff" => actions::diff(),
        "files" => actions::files(),
        "sync" => actions::sync(),
        "upload" => actions::upload(),
        "auto" => actions::auto(),
        "clean" => actions::clean(),
        "new" => actions::new(&args[2..]),
        "status" => actions::status(),
        "check" => actions::check(),
        _ => fail!("command `{}` not found", args[1]),
    }
}
