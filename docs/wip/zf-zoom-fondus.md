# ZF — fondus au zoom sur la carte de campagne

Plainte joueur (2026-10-10) : les éléments de la carte « se chargent brutalement » au zoom.
Audit (Explore) : la plupart des couches fondent déjà ; les apparitions brutales viennent de
bascules `visible` à seuil, de `visibility_range_end` sans marge ni fondu, de cellules semées
qui naissent à pleine taille et de changements de LOD sans fondu enchaîné.

## Lots
- **ZF-A** (fondus de transparence, mécanique) : hameaux (settlement_layer 863/1783), tuiles
  maquettes (town_maquette_layer 232/695, marge + FADE_SELF), water_props (48/290), bateaux
  life_ambient (214), moulins life_effects (775), oiseaux map_bird_flocks (398), passerelles
  fine_geo (533/573), bivouacs army_figures (647). Seuils booléens → poids lissés ZoomTiers +
  `GeometryInstance3D.transparency`.
- **ZF-B** (semis par cellule) : countryside / fauna — rampe d'apparition par cellule nouvelle
  et sortie en fondu au rayon de chargement ; fauna.gdshader rang binaire → `vis` lisse comme
  countryside.gdshader:47 ; rochers : chute LOD2 à 55 % en fondu (`pixel_fade`).
- **ZF-C** (LOD arbres/villes) : vegetation impostor↔détaillé en fondu tramé (étendre
  near_start/near_end), forest_detail cartes↔impostors, town_layer/landmark_city bascule à
  rig 45 en fondu.
- Hors périmètre : field_layer (désactivé), folk_pool (figurines proches, à voir après).

## État
- 2026-10-10 : audit fait, lots lancés.
- 2026-10-10 : ZF-A (8a267db46), ZF-B (3ccbabc86), ZF-C (9db88360a) commités sur leurs branches,
  fusionnés dans la branche `zf-merge` (worktree ../gp-zf-merge). Notes de lot :
  docs/wip/zf-a.md, docs/wip/zf-c-lod-fondus.md. Incident disque plein pendant les imports des
  agents (agents arrêtés). Prochaine étape : import + smoke + tests de couche, puis ff dans main.
- Laissé de côté : fondu enchaîné des LOD countryside/fauna (hystérésis seule), tramage
  `pixel_fade` des rochers (rampe par instance seulement), field_layer, folk_pool.
