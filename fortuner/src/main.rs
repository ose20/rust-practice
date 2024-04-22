use fortuner::{get_config, run};

fn main() {
    if let Err(e) = get_config().and_then(run) {
        eprint!("{}", e);
        std::process::exit(1);
    }
}
