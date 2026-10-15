# bm-bvis2 : lumière et ombres des batailles (client Bevy)

Branche `bm/bvis2` (suite de bvis). Méthode : captures hors écran comparées aux planches Godot `battle_low` / `battle_default`.

## Fait
- Lumière de caméra (`client/src/battle/scene/lighting.rs`, données `data/fx/battle_lighting.json`) : exposition EV100 11,05 (une unité d'énergie Godot = éclairement Godot ; avant : EV 9,7 par défaut, soleil 2,5 fois trop fort), tonemapping AgX (Godot), étalonnage (contraste x saturation des étalonnages saison / météo / heure, température), lumière du ciel HDRI (panorama Poly Haven de atmosphere.json rééchantillonné en cubemap selon hdri_sky.gdshader, filtré par `GeneratedEnvironmentMapLight`), part neutre en `AmbientLight` de caméra (couleur du brouillard éclaircie), SSAO si le MSAA de bataille est coupé (+ SMAA).

## Écarts assumés
- Halo (bloom) coupé : une caméra de bataille HDR sous la caméra du HUD non HDR donne une image noire.
- LUT d'étalonnage Godot : seuls contraste, saturation, température passent ; lift / gain / tons des ombres et hautes lumières non portés.

## Prochaine étape
Ombres portées des soldats instanciés (phase `Shadow`, `units/skinned.rs`).
