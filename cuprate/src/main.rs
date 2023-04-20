use tracing::{span, Level, info, event};

pub mod cli;

const CUPRATE_VERSION: &str = "0.1.0";

fn main() {
	// Collecting options
    let matches = cli::args();
	
	// Initializing tracing subscriber and runtime span
	let _runtime_span = cli::init(&matches);
}
