# bm-campveg5 : végétation et campagne (décor) — forêt, ombres, côtes

Branche `bm/campveg5` (base 42068a22b). Dossiers : `client/src/campaign/vegetation`, `client/src/campaign/countryside`.

## État
- Compile, clippy propre, `cargo test --features dev` vert (1170 + tests d'intégration).
- FAIT : vraies ombres portées des arbres (V9). `vegetation/shadow_pass.rs` + `shadow_cast.wesl` : les lots
  d'imposteurs et de modèles marqués `VegShadowCaster` sont dessinés en profondeur seule dans les cascades du
  soleil (phase `Shadow` de Bevy, éléments `NonMesh`, retirés/réajoutés à chaque image). Imposteur = carte
  verticale tournée vers l'azimut du soleil, vue d'atlas la plus proche ; découpe alpha, bande modèle/imposteur
  (Bayer) et hiver ajouré identiques à la passe principale ; vent suivi. Bascule par distance caméra
  (`vegetation/shadows.rs`) : 70 (`generalised_shadow_distance` Godot) + hystérésis 8 %, plafonnée par
  `veg_shadow_distance` du préréglage (`data/fx/render_quality.json`, bas = 0). Les ellipses au sol (vegvis)
  sont retirées. Mieux que Godot : le relief, les villes et les couronnes voisines reçoivent l'ombre réelle.
- FAIT : rochers de côte (`countryside/coast_rocks.rs`) : `coast_dist.png` + `coast_types.json`, port de
  `_coast_term`, `region_at`, `kind_at`/`shore_probe` (falaises) ; les entrées `coast` du catalogue
  (`rock_outcrops.yaml`) ne sont plus filtrées.
- FAIT : panaches de fumée du décor (`countryside/decor_smoke.rs`) : types `smoke` (forges, charbonniers…),
  colonne du flipbook fumée du joueur au sommet du modèle, coupée au-delà de 160 ou si l'instance est cachée.
  Mieux que Godot (bouffées qui montent et s'estompent au lieu d'un quad fixe).

## Non portés (décisions)
- V8 `forest_detail*.gd` : éteint dans le style d'arbres actif « generalised » (`data/ui/campaign_map.json`,
  `use_forest_detail and not generalised`) ; la densité par zoom du style généralisé est déjà portée
  (`render::density_at`). Rien à porter.
- Hiver ajouré `foliage_*_winter` : déjà présent dans `billboard.wesl` (imposteurs et modèles), repris dans
  la passe d'ombre.
- Roselières : déjà dessinées par `vegetation/reeds.rs` (touffes + mares des zones humides). ATTENTION à la
  fusion avec bm/campwater2 (`water/fresh.rs` dessine aussi les étangs) : risque de mares en double.
- `roofscape` / `building_regional` : servent aux villes 1:1 et au kit de repli (`campaign/towns`, hors lot) ;
  invisibles en style maquette par défaut (glb DN 392/392). Non applicable ; towns4 ne les a pas faits.
- Cartes de plantes TX T3 (`ground_cards.gd`) : le fouillis n'apparaît que sous une distance caméra de 2,6
  alors que le plancher caméra maquette est 20 ; atlas 7×10 × 1024 px (~290 Mo décodés). Reste.
- Cicatrices restantes : croix de portes = villes 1:1 (non applicable en maquette) ; engins de siège près du
  camp (`war_scars.json` bloc `siege`, maillages `timber_pile`/`engine_frame`/`scaffold`/`ladders`) : reste.

## Preuve (vidages hors écran, Paris xz 1651,2731, sans capture)
| vue | imposteurs | modèles | lots | ombres (lots projetés) |
|---|---|---|---|---|
| été d=25 | 23 464 | 35 | 25 | oui (25), cascades 87,5 |
| été d=60 | 34 509 | 138 | 34 | oui (34), cascades 210 |
| été d=200 | 67 983 (22 576 éclaircis) | 0 | 24 | non (> 70) |
| hiver d=60 (`CENT_ANS_SEASON=winter`, poids [0,0,0,1]) | 34 509 | 138 | 34 | oui (34) |

Aucune erreur de shader ni panique dans les journaux. Rochers : 5 entrées `coast` actives, raster chargé (rien
près de Paris, normal). Fumée : aucun type `smoke` dans les 19 décors vivants près de Paris (0 panache) ;
les trois types fumants (`charcoal_kiln`, `forge_water`, `glassworks`) sont vérifiés par test.

## Restes
- Cartes de plantes TX T3, engins de siège (cicatrices), vérification à l'œil des ombres (couleur, acné) et
  d'un panache près d'une charbonnière.
- Fusion avec bm/campwater2 : mares en double possibles (`vegetation/reeds.rs` / `water/fresh.rs`).

## Prochaine étape
- Lot terminé (marqueur `.bm-ready`).
