fn main() {
    if let Err(e) = catr::get_args().and_then(catr::run) {
        std::process::exit(1);
    }
}
