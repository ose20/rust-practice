fn main() {
    if let Err(e) = headr::get_config().and_then(headr::run) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
