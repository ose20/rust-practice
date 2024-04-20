fn main() {
    if let Err(e) = tailr::get_config().and_then(tailr::run) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
