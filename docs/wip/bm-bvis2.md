# bm-bvis2 : lumière et ombres des batailles (client Bevy)

Branche `bm/bvis2` (suite de bvis). Méthode : captures hors écran comparées aux planches Godot `battle_low` / `battle_default`.

## Fait
- Lumière de caméra (`client/src/battle/scene/lighting.rs`, données `data/fx/battle_lighting.json`) : exposition EV100 11,05 (une unité d'énergie Godot = éclairement Godot ; avant : EV 9,7 par défaut, soleil 2,5 fois trop fort), tonemapping AgX (Godot), étalonnage (contraste x saturation des étalonnages saison / météo / heure, température), lumière du ciel HDRI (panorama Poly Haven de atmosphere.json rééchantillonné en cubemap selon hdri_sky.gdshader, filtré par `GeneratedEnvironmentMapLight`), part neutre en `AmbientLight` de caméra (couleur du brouillard éclaircie), SSAO si le MSAA de bataille est coupé (+ SMAA).

- Ombres portées des soldats (`units/skinned.rs`) : chaque lot de figures est un élément « non-mesh » de la phase `Shadow` de Bevy dans chaque cascade du soleil, même étage de sommets (animation par texture d'os), disposition du prépassage (vue / vide / matière), profondeur seule, sans élimination des faces ; imposteurs exclus. Preuve : dump `battle_lighting.shadow_casters` (lots x cascades), `shadow_draws_total` ; différence d'images avec / sans (`CENT_ANS_NO_FIGURE_SHADOWS=1`).
- Les tirages instanciés (figures, armes, flèches) reçoivent désormais les ombres : `pbr_input_new()` laissait les drapeaux du maillage à 0, sans le bit `MESH_FLAGS_SHADOW_RECEIVER_BIT`.
- Sonde : `CENT_ANS_SHADOW_PROBE=1` suspend un cube et dresse un poteau près du pivot de la caméra.

## Écarts assumés
- Halo (bloom) coupé : une caméra de bataille HDR sous la caméra du HUD non HDR donne une image noire.
- LUT d'étalonnage Godot : seuls contraste, saturation, température passent ; lift / gain / tons des ombres et hautes lumières non portés.

## Prochaine étape
Lisière d'arbres proche, collines d'horizon (decor/horizon.rs, hors de mon dossier), vignes (site.rs).

## Restes
- L'herbe (`decor/grass.rs`, WindMaterial) ne reçoit visiblement pas les ombres : elle couvre le sol et masque celles des soldats (lot décor).
- Armes et boucliers (`instancing.rs`, partagé avec la végétation de campagne) ne projettent pas d'ombre.
