fn main() {
    if let Err(e) = findr::get_config().and_then(findr::run) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
