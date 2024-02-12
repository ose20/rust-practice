fn main() {
    if let Err(e) = cutr::get_config().and_then(cutr::run) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
