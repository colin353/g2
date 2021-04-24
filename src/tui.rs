pub fn select<S: ToString + std::fmt::Display + ?Sized>(
    prompt: &str,
    options: &[&S],
) -> Result<usize, ()> {
    if options.is_empty() {
        return Err(());
    }

    if options.len() == 1 {
        return Ok(0);
    }

    println!("{}", prompt);

    Ok(dialoguer::Select::new()
        .default(0)
        .items(&options)
        .interact()
        .unwrap())
}
