// SkullBlockEntityRendererMixin: a placed player head draws its hat in 3D.
fn skull(
    part: &crate::block_entity_render::ModelPart,
    name: &str,
    parent: glam::Mat4,
    adjust: &dyn Fn(&str, &mut [f32; 9]),
    paint: crate::block_entity_render::Paint,
    out: &mut Vec<crate::mesher::Vertex>,
) -> std::ops::ControlFlow<()> {
    let r = paint.sprite;
    match crate::skin_layers_3d_core::head_for(part, name, [r.u0, r.v0, r.u1, r.v1], false) {
        Some(model) => {
            crate::block_entity_render::draw_part(model, name, parent, adjust, paint, out);
            std::ops::ControlFlow::Break(())
        }
        None => std::ops::ControlFlow::Continue(()),
    }
}
