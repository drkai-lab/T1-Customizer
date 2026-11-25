fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(t1_keyboard_config::cli::run(&args));
}
