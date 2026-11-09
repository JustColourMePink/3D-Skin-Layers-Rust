// The first-person arm, with a 3D sleeve when that sleeve is on.
fn arm(
    game: &crate::game::Game<'_>,
    left: bool,
    matrix: glam::Mat4,
    light: u32,
    out: &mut Vec<horizon_client::mesher::Vertex>,
) -> std::ops::ControlFlow<()> {
    crate::game::skin_layers_3d::first_person_arm(game, left, matrix, light, out)
}
