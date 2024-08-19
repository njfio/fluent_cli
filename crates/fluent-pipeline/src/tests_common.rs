use tracing_subscriber;

#[cfg(test)]
pub fn init() {
    let _ = tracing_subscriber::fmt::try_init();
}
