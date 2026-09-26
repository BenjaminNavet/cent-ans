# FG3 — Matières cuites des figurines fines

Branche : `feat/fg3-materials` (worktree agent `agent-aba08b13133c215dc`). Plan :
`docs/wip/fg-figurines-fines.md`. Précédents : `fg0-prototype.md` (cuisson test),
`fg1-corps.md`, `fg2-equipement.md`, `fg4-cheval.md`.

## État : EN PAUSE (demande du joueur, 26/09) — code et cuisson complets, vérifs à finir
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
- [ ] Captures `docs/img/fg/fg3_*.png`, smoke + DA1/EP12 avec et sans drapeau, banc
  `--units=50`, réglages de goût, ADR, `battle_fine/SOURCE.md`, journal de l'orchestration

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
- Camail visible (cape de mailles sur le surcot), mailles claires lisibles.
- Barbe : plus de masque noir, bords fondus dans la peau ; teintes `FG3_HAIR` ; léger liseré
  géométrique à la lèvre (bord de la coque).
- Martelage des casques et bois adoucis une fois (casques 0,45) : à revoir en capture.

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
godot --path game res://scenes/battle/battle.tscn -- --fine-figures --screenshot=<png> --closeup
godot --path game res://scenes/battle/battle.tscn -- --units=50 --benchmark --bench-at=90 [--fine-figures] [--no-fg3]
blender -b --factory-startup --python tools/blender_scripts/battle_fine.py -- bake [--only a,b]
```

## Prochaine étape
Captures défaut | `--fine-figures` (gros plan, mêlée 6/12/26 m, archers, charge, armoiries
de maison via `da1_arms_shot.gd`), smoke, `ep12_shot.gd`, banc avec/sans drapeau, ADR (numéro
libre à vérifier, ~0088), `battle_fine/SOURCE.md` (textures), journal de
`fg-figurines-fines.md`.
