// A skin decoded into the downloader: a copy for the head renderers, which can't reach it.
fn remember(location: &str, rgba: &[u8]) {
    crate::skin_layers_3d_core::remember(location, rgba);
}
