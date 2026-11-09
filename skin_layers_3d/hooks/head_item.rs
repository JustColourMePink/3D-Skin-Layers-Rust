// A player head item (held, in a slot, dropped, worn: CustomHeadLayerMixin) draws its hat in 3D.
fn head_item(
    part: &crate::block_entity_render::ModelPart,
    name: &str,
    parent: glam::Mat4,
    adjust: &dyn Fn(&str, &mut [f32; 9]),
    sprite: crate::hud::Rect,
    color: u32,
    foil: crate::item_render::Foil,
    sink: &mut dyn crate::item_render::ItemSink,
) -> std::ops::ControlFlow<()> {
    let r = sprite;
    let rect = [r.u0, r.v0, r.u1, r.v1];
    match horizon_client_render::skin_layers_3d_core::head_for(part, name, rect, true) {
        Some(model) => {
            draw(model, name, parent, adjust, sprite, color, foil, sink);
            std::ops::ControlFlow::Break(())
        }
        None => std::ops::ControlFlow::Continue(()),
    }
}
