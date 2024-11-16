use tokio::task::{spawn_blocking, JoinHandle};
use tracing::{subscriber, Level};
use tracing_subscriber::FmtSubscriber;

pub fn get_subscriber() -> FmtSubscriber {
    FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_line_number(true)
        .with_target(true)
        .finish()
}

pub fn init_subscriber(subscriber: FmtSubscriber) {
    subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}

pub fn spawn_blocking_with_tracing<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let currect_span = tracing::Span::current();
    spawn_blocking(move || currect_span.in_scope(f))
}
