fn main() {
    if let Err(e) = catr::get_config().and_then(catr::run) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
