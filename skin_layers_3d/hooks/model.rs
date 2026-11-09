// Horizon's 3D skin layers model: ours, built the way 3D Skin Layers builds it.
fn model(
    game: &crate::game::Game<'_>,
    kind: &std::sync::Arc<horizon_client::mobs::MobType>,
    skin: &str,
    slim: bool,
) -> std::ops::ControlFlow<Option<std::sync::Arc<horizon_client::mobs::Part>>> {
    std::ops::ControlFlow::Break(crate::game::skin_layers_3d::model_for(game, kind, skin, slim))
}
