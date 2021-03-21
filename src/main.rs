#[macro_use]
mod fail;

mod cmd;
mod conf;

fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 2 {
        fail!("you need to provide at least one argument!");
    }
    match args[1].as_str() {
        "clone" => cmd::clone(args[2].as_str()),
        "branch" => cmd::branch(args[2].as_str()),
        _ => fail!("command `{}` not found", args[1]),
    }
}
