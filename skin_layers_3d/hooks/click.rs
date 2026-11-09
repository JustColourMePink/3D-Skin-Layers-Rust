// A settings button was clicked.
fn click(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
    crate::game::skin_layers_3d::click(game, mod_id, button);
}
