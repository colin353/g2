pub fn select<S: ToString + std::fmt::Display + ?Sized>(options: &[&S]) -> Result<usize, ()> {
    if options.len() == 0 {
        return Err(());
    }

    if options.len() == 1 {
        return Ok(0);
    }

    Ok(dialoguer::Select::new()
        .default(0)
        .items(&options)
        .interact()
        .unwrap())
}
