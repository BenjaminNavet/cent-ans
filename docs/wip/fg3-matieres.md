# FG3 — Matières cuites des figurines fines

Branche : `feat/fg3-materials` (worktree agent `agent-aba08b13133c215dc`). Plan :
`docs/wip/fg-figurines-fines.md`. Précédents : `fg0-prototype.md` (cuisson test),
`fg1-corps.md`, `fg2-equipement.md`, `fg4-cheval.md`.

## État : TERMINÉ 26/09 (branche prête, non fusionnée dans main) — main fusionné (4c89ce24, sans conflit), smoke OK, fg3_maps OK
- [x] Commande `bake` (`battle_fine.py -- bake [--only a,b]`, `all` = rigs + bake), format
  `CAM2` (UV d'atlas), variante `FG3_BAKED` du shader, chargement des cartes dans
  `BattleSkinned`, `--no-fg3` (figurines fines sans cartes, A/B)
- [x] Tuiles de détail (`battle_fine_tiles.py`, numpy) : mailles, tissage, feutre, cuir, acier
  martelé, bois, peau, cheveux ; 512² × 8, Texture2DArray BC7
- [x] Atlas par figurine : LOD0 512², LOD1 256² (bandes Texture2DArray, couche = rang de la
  recette dans `FIGURES`) : RG normale de forme (sources HD = pièces avant décimation + plis
  subdivisés des étoffes), B AO par groupe de variantes, A masque (densité barbe/cheveux,
  sourcils, martelage : casques 1, autres plates 0,35)
- [x] Cheval : `fine_horse.png` 1024² (normale CC0, relief de la robe, AO), partagé
- [x] Shader : branches FG3 (atlas + tuiles selon le code matière), LOD2 sans lecture ni dérivée ;
  imposteurs avec cartes (`fine_distance` levé dans `battle_impostors.gd`)
- [x] Camail : faces tournées vers l'intérieur depuis FG0 (normales bmesh jamais calculées
  avant le test d'orientation) : corrigé dans `battle_fine_equipment.aventail` (et deux
  bardes de `battle_fine_cavalry`) ; le camail est maintenant visible et clair
- [x] Cuisson des 28 recettes faite et commitée (bandes, maillages `CAM2`, `atlas_layer` au
  manifeste) ; `.import` écrits par le script (Godot complète uid et chemins)
- [x] Rendu par défaut identique au pixel près (vivants et cadavres ; shader et
  `battle_skinned.gd` de main contre ceux de la branche, `v2_figures_shot` infantry_0 + cavalry_1)
- [x] Mémoire mesurée (`tests/fg3_maps_test.gd`, BC7) : atlas LOD0 9,33 Mo + LOD1 2,33 +
  tuiles 2,67 + cheval 1,33 = **15,7 Mo** (budget 60)
- [x] Captures `docs/img/fg/fg3_*.png` (défaut | FG3 | `--no-fg3`) : `fg3_gros_plan_infanterie`,
  `fg3_cavalier`, `fg3_melee_26m` (bataille `--closeup`) ; défaut | FG3 : `fg3_armoiries_da1`,
  `fg3_blesses_ep12`, `fg3_fuyards_ep12` (tout s'affiche ; fuyards identiques à `--no-fg3`)
- [x] Bogue corrigé : le drapeau porté d'EP5 disparaissait sous FG3 (`BattleStandards` appelle
  `setup_material` sur un matériau `battle_standard_flag` que `_setup_fine_maps` basculait sur le
  shader skinné) : seuls les matériaux au shader skinné (ou une de ses variantes) basculent
- [x] Réglages de goût (voir plus bas), banc `--units=50`, ADR 0088, `battle_fine/SOURCE.md`

## Choix d'architecture
- UV d'atlas empaquetée dans UV2.y (11 bits u, 11 bits v, 2 bits source : 1 atlas LOD0,
  3 atlas LOD1, 2 cheval) : aucun attribut de sommet en plus ; LOD2 `CAM1` : 0.
- Repère tangent reconstruit au fragment (dérivées écran) : pas de tangentes stockées.
- Variante `#define FG3_BAKED` (comme `BV2_CORPSE`) posée par `BattleSkinned.setup_material`
  pour les figurines fines cuites seulement ; `corpse_shader()` rend la variante cadavres + FG3
  sous `--fine-figures`.
- Îlots d'atlas tirés des UV propres des pièces (MakeHuman, FG2) ; projection intelligente pour
  les pièces sans UV ; hampes longues plafonnées (`LONG_ISLAND`), têtes ×1,8.
- Écart à la spec : atlas par figurine en 512² (et non 1024² par famille) : même mémoire
  (4 figurines = 1 atlas 1024²), sans partage d'UV entre recettes différentes.

## Réglages vus en gros plan (v2_figures_shot)
- Camail visible (cape de mailles sur le surcot).
- Barbe : plus de masque noir, bords fondus dans la peau ; teintes `FG3_HAIR` ; léger liseré
  géométrique à la lèvre (bord de la coque).
- Reprise 26/09 : plates (casques, canons) « papier froissé » : venait surtout de l'AO cuite
  (bruit Cycles sur les grandes pièces lisses) et un peu de la normale de forme. Plates et
  garnitures : AO ramenée à `mix(0.7, 1, smoothstep(0.25, 0.85, ao))`, normale de forme ×0,35,
  martelage 0,05-0,22 (garnitures 0,1). Les bosses restantes sont géométriques (aussi en
  `--no-fg3`).
- Mailles trop claires et plates, confondues avec l'acier : albédo `mix(0.18, 1.2, b²)`, tuile
  0,072 -> 0,1 m (anneaux ~12 mm) ; la maille se lit maintenant plus sombre que les plates.

## Banc `--units=50 --benchmark --bench-at=90` (1600×900, ~11 960 soldats, 4 passes alternées)
| | défaut | `--fine-figures` (FG3) | `--fine-figures --no-fg3` |
|---|---|---|---|
| primitives (M) | 3,14 | 3,49 | 3,50 |
| i/s moyens | 38,6 | 36,5 | 38,2 |
| image médiane (ms) | 26,7 | 29,1 | 26,3 |
| p95 (ms) | 33,6 | 37,3 | 32,7 |

Bruit d'une passe à l'autre ±20 % (i/s 26-50 pour une même configuration) ; tendance : FG3
≈ +2,5 ms en médiane (≈ 9 %) sur les figurines fines sans cartes, qui coûtent autant que le
défaut. Piste FG5 : `fine_distance` plus court (80 -> 40 m) ou tuiles seulement au LOD0.

## Pièges
- `bake --only` en parallèle : verrou `textures/.lock` (ne pas commiter) ; manifeste relu
  avant écriture.
- Pièces de variante superposées (têtes, casques) : AO par groupe de bits de variante,
  normale HD pièce par pièce (sélection vers active).
- `godot --headless` ne relit pas les textures compressées : `fg3_maps_test.gd` calcule la
  taille d'après le format.
- Le `.blend` du cheval CC0 (20 Mo) est retéléchargé dans le worktree (ignoré par git).
- Cuisson complète ≈ 25 min (28 recettes, montés les plus longs).

## Reprise (commandes)
```
cp /Users/jean_hubert/dev/game_project/game/bin/*.dylib game/bin/
godot --headless --path game --import
godot --headless --path game --script res://tests/fg3_maps_test.gd -- --fine-figures
godot --path game --resolution 1600x900 --script res://tests/v2_figures_shot.gd -- \
  --fine-figures [--no-fg3] --out=<png> --fig=infantry_0 --cols=3 --rows=1 --cam=1.6,1.6,2.6,1.1,1.2,0
godot --path game res://scenes/battle/battle.tscn -- --fine-figures --screenshot=<png> --closeup --no-hud
# zsh : ne pas passer les options dans une variable ($F n'est pas découpé), les écrire en clair
godot --path game res://scenes/battle/battle.tscn -- --units=50 --benchmark --bench-at=90 [--fine-figures] [--no-fg3]
blender -b --factory-startup --python tools/blender_scripts/battle_fine.py -- bake [--only a,b]
```

## Prochaine étape
Fusion dans main par l'orchestrateur ; puis FG5 (perf A/B, `--fine-figures` par défaut ?).
Pistes : coutures des UV projetées de très près, liseré barbe/lèvre, coût FG3 (voir banc).
