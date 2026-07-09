fn main() {
    adaptive_log::init();

    log::error!("something went wrong");
    log::warn!("watch out");
    log::info!("up and running");
    log::debug!("detail");
    log::trace!("verbose");
}
