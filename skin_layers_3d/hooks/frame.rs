// Once a frame: level of detail by distance, and models the skin update could not build yet.
fn frame(game: &mut crate::game::Game<'_>) {
    crate::game::skin_layers_3d::frame(game);
}
