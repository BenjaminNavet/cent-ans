# bm-bground : sol et arbres de bataille (V14, V15)

Branche `bm/bground` (base 42068a22b). Fichiers : `client/src/battle/decor/ground.rs`, `ground.wesl`,
`trees.rs`, `tree_kit.rs` (nouveau : squelette des arbres et textures d'écorce / de feuilles).
Hors dossier, modification minimale : `battle/scene/terrain.rs` (appel de
`ground::build_ground_material`, paramètre `images`), `battle/decor/mod.rs` (`pub mod tree_kit;`).

## État
- Sol : `ground.wesl` réécrit (port de `battle_ground.gdshader` + `battle_common.gdshaderinc`) : pile
  par priorité des 13 couches, normales du paquet, double échantillonnage, aplanissement au loin,
  détail de près DA6, grain fin PO4 (`micro_ground`), parcelles EP6 cuites à 4 m (`decor_fields`),
  parcelles procédurales hors du centre calme, relief R2 cuit sur la grille du champ (pente, creux,
  crêtes) + bosses de détail, neige B7 / boue piétinée B8, humidité, désaturation. Teinte de saison,
  neige et humidité : port de `battle_terrain.gd` `_build_material`. Vidage `battle_ground`.
  Le shader est validé par naga dans un test (`ground_shader_validates`).
- Arbres : `tree_kit.rs` (7 essences de `battle_trees.gd`, squelette récursif, bouquets, 2 maillages
  complet / allégé, planches TX `tx_bark_pack` / `tx_leaves_pack` découpées, hiver : ramilles nues
  `twig_spray.png`, chêne marcescent `dead_leaves_oak.png`), `trees.rs` : 3 niveaux = complet
  (< 120 m, buissons < 60 m, fondu 16 m), allégé, puis imposteurs `far_woods`. Vidage `battle_trees`.
- Tests, clippy : verts. Reste : vidages été / hiver (scripts dans le scratchpad).

## Écarts à la spec
- Piétinement neige / boue : réutilise la carte d'herbe couchée de `flatten.rs` (pas de 2e carte).
- Table BIOMES et constantes de bruit de Godot portées en constantes Rust / WGSL ; le bruit macro
  est un fbm procédural au lieu de la texture `macro_noise`.
- Arbres : paquets TX en qualité Medium (1024) ; LOD par `VisibilityRange`.

## Restes
- Micro-écorce (`tx_micro_bark.jpg`, MICRO_FADE) et flou mip de l'écorce lisse non portés.
- Herbe FA7 blé / chaume (`grass.rs` inchangé).
- Variantes 2048 des paquets (qualité Haute).
