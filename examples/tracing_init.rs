fn main() {
    adaptive_log::init_tracing();

    tracing::error!("something went wrong");
    tracing::warn!("watch out");
    tracing::info!(user = "tamer", "up and running");

    let span = tracing::info_span!("request", method = "GET", path = "/users/42");
    let _guard = span.enter();
    tracing::debug!("handling inside the span");
}
