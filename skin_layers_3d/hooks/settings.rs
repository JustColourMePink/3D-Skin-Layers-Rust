// The settings page's buttons.
fn settings(game: &mut crate::game::Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    crate::game::skin_layers_3d::settings(game, mod_id, buttons);
}
