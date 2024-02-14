fn main() {
    if let Err(e) = grepr::get_config().and_then(grepr::run) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
