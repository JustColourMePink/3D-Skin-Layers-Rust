# 3D Skin Layers 1.11.4 (NeoForge, MC 26.3): how it works

Reverse-engineered from `skinlayers3d-neoforge-1.11.4-mc26.3.jar` with `javap`.
Original by tr7zw, under the **tr7zw Protective License**: use, modify and compile
allowed; **no commercial advantage or monetary compensation**; the copyright and
licence notice must be kept with any copy. The port must ship that notice.

## Config (defaults)

| field | default | meaning |
|---|---|---|
| enableHat / Jacket / LeftSleeve / RightSleeve / LeftPants / RightPants | true | per-part 3D toggle |
| baseVoxelSize | 1.15 | X/Z scale of limb meshes (third person) |
| bodyVoxelWidthSize | 1.05 | X scale of the torso mesh |
| headVoxelSize | 1.18 | uniform scale of the head mesh |
| renderDistanceLOD | 14 | blocks; beyond this the vanilla flat layer is used |
| enableSkulls / enableSkullsItems | true | 3D player-head blocks / items |
| skullVoxelSize | 1.1 | uniform scale for skulls |
| fastRender | true | draws more faces, skips transparency sorting work |
| firstPersonPixelScaling | 1.1 | X/Z scale for first-person arms |
| compatibilityMode, irisCompatibilityMode, applySodiumWorkaround | true/false/false | Java-renderer workarounds; **not needed in Horizon** |

## Meshes built per skin (`SkinUtil.setup3dLayers`)

Skin must be 64×64. `create3DMesh(w, h, d, u, v, topPivot, yOffset)`:

| part | w,h,d | texture u,v | topPivot | offset |
|---|---|---|---|---|
| head (hat) | 8,8,8 | 32,0 | false | 0.6 |
| torso (jacket) | 8,12,4 | 16,32 | true | 0 |
| left arm (sleeve) | 4 (slim 3),12,4 | 48,48 | true | -2 |
| right arm (sleeve) | 4 (slim 3),12,4 | 40,32 | true | -2 |
| left leg (pants) | 4,12,4 | 0,48 | true | 0 |
| right leg (pants) | 4,12,4 | 0,32 | true | 0 |

Rebuilt only when the skin id or slim flag changes. Texture lookups are cached 60 s.

## Voxel builder (`SolidPixelWrapper.wrapBox`)

Box origin: `x = -w/2`, `z = -d/2`, `y = topPivot ? offset : -h + offset`.
Standard box UV layout at (u,v) (same as a vanilla cube unwrap):

| face | face size (U×V) | on-texture origin |
|---|---|---|
| DOWN | w×d | (u+d, v) |
| UP | w×d | (u+d+w, v) |
| NORTH | w×h | (u+d, v+d) |
| SOUTH | w×h | (u+d+w+d, v+d) |
| WEST | d×h | (u, v+d) |
| EAST | d×h | (u+d+w, v+d) |

For every face, for every texel (fu,fv) in it:
- `present` = alpha != 0; `solid` = alpha == 255. Skip the texel if not present.
- Voxel position from face UV (`UVtoXYZ`):
  - DOWN `(fu, 0, d-1-fv)`; UP `(fu, h-1, d-1-fv)`; NORTH `(fu, fv, 0)`;
    SOUTH `(w-1-fu, fv, d-1)`; WEST `(0, fv, d-1-fu)`; EAST `(w-1, fv, fu)`
- Inverse (`XYZtoUV`): DOWN/UP `(x, d-1-z)`; NORTH `(x, y)`; SOUTH `(w-1-x, y)`;
  WEST `(d-1-z, y)`; EAST `(z, y)`.
- Neighbour test for each direction `n` whose axis != the face's axis:
  - `p1 = voxel + n`. If p1 is still on this face:
    - if this face has a texel at p1 (and, when this voxel is solid, that texel
      is solid too) → **hide side n** (merged with neighbour).
    - else look at `p2 = p1 + n`; if p2 is off this face, map p2 onto the
      **face in direction n** and, if a texel is there (solid-matched), hide n.
  - If p1 is off this face (voxel is on the face's edge):
    - `onEdge = true`. If the adjacent face `n` has a texel at this voxel's
      position: `hasNeighbourFace = true`, hide n, and record a **corner pair**
      `[face.opposite, n]`.
    - else if the adjacent face has a texel at `voxel - face.step`: `hasNeighbourFace = true`.
- After the loop: if `!onEdge || hasNeighbourFace`, hide `face.opposite` (the inward side).
- If `fastRender`, the bytecode also adds `face` itself. Ported literally, that hid
  the **outward** face in Horizon (confirmed in game: hollow layers). The port
  instead makes Fast Render hide the inward face (`face.opposite`) on every voxel,
  edge voxels included; that's the cheap path the option is meant to be.
- Emit a 1×1×1 cube at `origin + voxel`, textured entirely with the single
  texel `(texU, texV)` on every face (UV span of one texel), with the hidden
  faces and corner pairs.
- `mirror` flips X (not used for players).

## Cube geometry (`CustomizableCube`)

8 corners of the unit cube; faces in the vanilla order DOWN, UP, NORTH, SOUTH,
WEST, EAST. For **corner connection**: for each corner pair, per axis, the
first direction whose axis matches is chosen; the face on that axis has the vertex
furthest along that direction removed, so the quad becomes a triangle
(`v[3] = v[2]` after compaction). That bevels outward corners of the layer
so edge voxels join diagonally instead of leaving notches.

## Placement at render time (`OffsetProvider`)

Rendered inside the vanilla layer ModelPart's transform (pivot + rotation),
replacing that part's vanilla cubes when the mesh is present:
- non-head: `scale(xScale, 1.035, zScale)`, where xScale = zScale = baseVoxelSize
  (first-person: firstPersonPixelScaling); BODY uses `bodyVoxelWidthSize` for X.
  Mesh position offset (pixels): x = ARMS 0.998 / ARMS_SLIM 0.499 (negated for the
  right arm), y = shape yOffset (BODY -0.2, LEGS -0.2, ARMS -0.1), z = 0.
- head: translate(0,-0.25,0); scale(headVoxelSize) (skull: skullVoxelSize);
  translate(0,0.25,0); translate(0,-0.04,0). Units here are **blocks** (PoseStack).

## When the 3D layers are used

Per frame in `PlayerModel.setupAnim`:
- Reset all injected meshes.
- Skip if the player is further than `renderDistanceLOD` blocks from the camera
  (squared-distance compare), so the vanilla flat layer shows.
- Build/refresh meshes; then inject per enabled part. The hat is skipped if the
  head slot holds an item in `hideHeadLayers` (helmets that cover it).
- A vanilla layer part that is hidden (`visible = false`) stays hidden.
- First person: same, for the visible arm's sleeve, using the FIRSTPERSON offsets.

## Port notes: heads (0.2.0)

- Player heads use the hat mesh (8,8,8 at 32,0, offset 0.6) with `OffsetProvider.SKULL`:
  the head placement with `skullVoxelSize` instead of `headVoxelSize`.
- Horizon draws placed skulls in `BlockEntityRenderer::render_one` and head items in
  `item_render::special::submit`; both contain closures, which horizon-mixins' site hooks
  (MODIFY_ARG, WRAP_OPERATION) refuse. So the port swaps the model at the draw functions
  instead (`block_entity_render::draw_part`, `special::draw`: cancellable HEAD), spotting a
  player head's root (`head` → `hat`) and its skin by the atlas sprite it is painted with.
- The skin images live in `skins::Downloader` (render crate), out of the renderers' reach:
  a HEAD hook on `Downloader::remember` mirrors them into `skin_layers_3d_core`, and the
  game's frame hook maps each skin's atlas sprite back to its location every 30 frames.
- Worn heads go through the item renderer (`DisplayContext::Head`), so the item hook covers
  `CustomHeadLayerMixin` too.
- `hideHeadLayers`: zombie, creeper, dragon, skeleton and wither skeleton heads keep the
  wearer's hat flat (player heads don't).

## Also in the original (later phases)

- Player head blocks and items (skulls) in 3D (`SkullBlockEntityRendererMixin`,
  `BlockEntityWithoutLevelRendererMixin`, `CustomHeadLayerMixin`).
- Deadmau5 ears, EMF compat, public API for other mods: out of scope.
- Config screen with three tabs: Players / Player Heads / Other Settings.
