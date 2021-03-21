macro_rules! fail {
    ($($tts:tt)*) => {
        {
            println!($($tts)*);
            std::process::exit(1);
        }
    }
}
