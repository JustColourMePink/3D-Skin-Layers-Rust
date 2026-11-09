# 3D Skin Layers for Horizon

Makes the outer layer of Minecraft skins pop out in 3D, so hats, jackets, sleeves and pants look
like real 3D pieces instead of flat stickers.

This is a version of tr7zw's popular [3D Skin Layers](https://modrinth.com/mod/3dskinlayers) mod,
rebuilt for [Horizon](https://playhorizon.dev) (a version of Minecraft Java Edition 26.3 rewritten
in Rust). All credit for the original idea and design goes to **tr7zw**
([original source](https://github.com/tr7zw/3d-Skin-Layers)).

> **Found a bug?** Please report it [here](../../issues), not to tr7zw. This port is a separate
> project.

## What it does

- **3D skins:** the hat, jacket, sleeves and pants layers of every player's skin become 3D. You
  can turn each part on or off.
- **Your own arms too:** your sleeves look 3D in first person.
- **3D player heads:** player heads look 3D, whether placed on the ground, held in your hand or
  worn by a player or mob.
- **Smart hat hiding:** when someone wears a mob head (zombie, creeper, skeleton and so on), their
  hat layer goes flat so it doesn't poke through.
- **Stays fast:** players far away switch back to the normal flat skin.
- **Adjustable:** change the sizes, the distance and more in
  **Options → Mods → 3D Skin Layers**.

## Installing

**The easy way:** in the Horizon launcher, open **Mods**, pick **Horizon** and install
*3D Skin Layers*.

**By hand:**

1. Download `skin_layers_3d.hnmod` from the [latest release](../../releases/latest).
2. Put it in `%APPDATA%\horizon\horizon-native-mods\`.
3. Turn the mod on in the launcher.
4. Rebuild the game so the mod is included:
   `horizon-launcher native-build --trust-native-code`

> **Heads up:** like every Horizon native mod, this one runs with the same access to your computer
> as the game itself. Only install native mods from people you trust. The full source is right here
> if you want to check it.

## Good to know

- Players using the default Steve or Alex skin stay flat, as do player heads with no skin attached.
- A player head you haven't seen before can take about half a second to become 3D.

## Licence

This mod uses the **tr7zw Protective License**, the same licence as the original (see
[LICENSE](LICENSE)). In short:

- ✅ You can use it, change it and build it yourself, for free.
- ❌ You can't sell it or use it to make money.
- 📄 If you share a copy, the licence must stay with it.

---

## For developers

<details>
<summary>How the project is laid out and how to build it</summary>

### Files

| Path | What it is |
|---|---|
| `skin_layers_3d/horizon-mixin.toml` | The mod's manifest: id, version, the Horizon version it targets and its hooks |
| `skin_layers_3d/src/core.rs` | Builds the 3D voxel models; settings, skin cache and player-head models |
| `skin_layers_3d/src/runtime.rs` | Runs every frame; first-person arms and the settings page |
| `skin_layers_3d/hooks/` | The small functions that hook into Horizon's code |
| `package.py` | Packs everything into `dist/skin_layers_3d.hnmod` |
| `NOTES-original-mod.md` | Notes on how the original mod works and where this port differs |

### Building

You need Python 3.11 or newer:

```
python package.py
```

This creates `dist/skin_layers_3d.hnmod`. A native mod file *is* its source code: the Horizon
launcher compiles it into the game on the player's computer. The same source always produces the
exact same file.

### Compatibility

Minecraft 26.3, Horizon revision `66f3981f522b24c250b21523a70080965b940424`
(horizon-mixins `5a64af938ec86b00d53d16a408aabc0038fb3e57`).

</details>
