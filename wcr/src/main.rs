fn main() {
    if let Err(e) = wcr::get_config().and_then(wcr::run) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
