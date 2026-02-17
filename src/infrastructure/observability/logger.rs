use tracing_subscriber::{EnvFilter, fmt, prelude::*};

pub fn init_tracing(log_level: &String) {
    let env_filter = EnvFilter::new(log_level);

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(env_filter)
        .init();
}
