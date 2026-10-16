# bm-fg3 — cartes fines FG3 des figurines (Bevy)

Branche bm/fg3 (sur bm/bherald), dossier `client/src/battle/units/`.

## État
- `fine_maps.rs` : charge (thread) les PNG de `models/battle_fine/textures/` : atlas lod0/lod1 (32 couches 512²/256²),
  `fine_detail_ga1` (12 couches) + `fine_detail_albedo`, `fine_horse` (1024²) ; mipmaps, ancrage répété (détail, cheval) ou calé (atlas).
  `fine_detail.png` (8 couches) n'est lu que sans la paire GA1, comme Godot. Dump `battle_fine_maps`.
- `figures.rs` : l'UV d'atlas empaquetée de `CAM2` devient `ATTRIBUTE_ATLAS` (location 3 ; zéros pour `CAM1` et l'imposteur) ;
  `atlas_layer` du manifeste -> `FigureSpec.atlas_layer` -> `SkinParams::set_fine` (vec4 `fine` du bloc, 127 vec4 au total).
- `skinned.rs` : liaisons 6-12 (atlas0, atlas1, détail, albédo de détail, cheval, 2 échantillonneurs) ; tant que les cartes ne sont pas
  envoyées au GPU le groupe est lié aux repli avec `layer = -1`, puis reconstruit.
- `skinned.wesl` : bloc FG3 du shader Godot : normale de forme (cotangente par dérivées), AO cuite, tuiles de détail GA1 par matière,
  albédo de détail, mailles, barbes, peau, pelage du cheval (poil couché), usure SR2 (boue, crasse, acier, rouille) ; LOD0 = atlas 1,
  LOD1 = atlas 3 (source portée par le maillage), LOD2/imposteurs = rien ; coupure à 80 m.

## Écarts à Godot
- `steel_occlusion` figé à 1 (valeur par défaut Godot), pas de lance (`v_prop`), pas d'`RADIANCE` d'occlusion du ciel sur la maille.
- Normales en espace monde (Godot : vue) ; formule invariante.
- Sampler : 2 échantillonneurs partagés (atlas / répété) pour rester sous la limite d'échantillonneurs de Bevy.

## Restes
- Œil : aucune capture (consigne), à regarder en jeu (maille, cheval).
- Dump montre `mesh_atlas_sources` par lot : 12 m -> [1,2], 45 m -> [2,3].

## bunits6 (suite)
- `steel_occlusion` est un vrai paramètre (`fine.z`, `SkinParams::set_steel_occlusion`, défaut 1.0 comme Godot, qui ne le surcharge jamais) : toutes les expressions `mix(.., steel_occlusion)` de Godot sont portées.
- Lances : `prop_*` du manifeste -> `FigureSpec.lance` (os `Prop` du rig) -> 4 vec4 `prop*` du bloc (131 vec4). Sommet : longueur, angle, flamme avant skinning ; `v_prop` -> coordonnées de fût, prise usée.
- Occlusion du ciel : pas de RADIANCE en Bevy ; équivalent `specular_occlusion` (lumière indirecte seule) ; maille rasante : metallic + reflectance 0.2 (SPECULAR 0.08 de Godot).
- Normales : cadre T/B résolu des dérivées avec signe du déterminant explicite (indépendant du sens de dpdy) ; miroir Rust testé (`tangent_space_normals_follow_the_uv_direction`). Écart : Godot (Schüler) dépend du sens de dFdy.
- Imposteurs > 300 m : armoiries (nobles) ou croix de livrée (communs) sur un patch de torse (amélioration, Godot n'en peint pas).
- Reste : oeil (maille, plates, lance, ciel), pas de test GPU du shader.
