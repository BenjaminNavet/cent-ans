# WIP S3 — Icônes des régimes alimentaires et `res_tin`

Branche : `worktree-agent-a4eb500d6919a0a8a`. Plan : `docs/wip/historien-suite.md` (lot S3).

## État : terminé

- [x] 7 icônes de régime (`data/diets/diet_*.json`) ajoutées à `tools/cent_ans_tools/icons_catalog.py`
      (catégorie `resource`, comme lue par `table_section.gd`) :
  - `diet_bread_pottage` → `lorc/cauldron` (marmite/potage)
  - `diet_dairy` → `lorc/cheese-wedge` (fromage)
  - `diet_lenten_fish` → `delapouite/fish-smoking` (poisson fumé/salé, Carême)
  - `diet_meat_salting` → `delapouite/bacon` (salaison)
  - `diet_pulses` → `delapouite/peas` (pois/fèves)
  - `diet_spiced_table` → `lorc/hot-spices` (sachet d'épices)
  - `diet_wine_bread` → `lorc/wine-glass` (coupe de vin)
- [x] Icône de la future ressource étain : `res_tin` → `faithtoken/ore` (minerai, distinct de
      `res_iron` → `lorc/metal-bar`). Nouvel auteur `faithtoken` ajouté à `AUTHORS`
      (crédité « Faithtoken » sur game-icons.net).
- [x] `diets` ajouté à `EXPLICIT_DATA_DIRS` (catégorie `resource`) : `missing_icons()` exigera
      désormais une icône dédiée pour tout futur régime.
- [x] `CREDITS.md` mis à jour (Delapouite 52, Lorc 59, + ligne Faithtoken).
- [x] Catalogue reconstruit : `uv run --project tools cent-ans assets icons` → 168 identifiants,
      124 SVG, 8 nouveaux fichiers `.svg`/`.import` dans `game/assets/icons/`.
- [x] Aucun changement GDScript nécessaire : `table_section.gd` (section « La Table ») utilise déjà
      `IconLibrary.get_icon(diet_id, "resource")` / `decorate_button(button, id, ..., "resource")`
      pour la puce du régime courant et les boutons d'options — l'identifiant du régime est déjà la
      clé d'icône, comme les autres entités. Avant ce lot, tous les régimes retombaient sur le repli
      générique `cat_resource` (caisse en bois) faute d'entrée dédiée.
- [x] Tests : `uv run --project tools pytest -q` → 87 passés (dont `test_icons.py`, 9 tests).

## Prochaine étape

Fusion par l'orchestrateur (voir méthode dans `docs/wip/historien.md`). `res_tin.json` sera ajouté
par l'agent S2 (autre branche) : l'icône est déjà prête, `missing_icons()` l'exigera automatiquement
dès que `data/resources/res_tin.json` existera (repli `resources` dans `EXPLICIT_DATA_DIRS`).

Capture d'écran de la section « La Table » non prise : la dylib `core/build.sh` n'était pas
disponible dans ce worktree et sa reconstruction dépasse le lot (voir note ci-dessous) ; le smoke
Godot suffit à vérifier l'intégration.

## Notes

- Cache des SVG bruts : `~/.cache/cent-ans/game-icons` (partagé entre worktrees).
- Sources vérifiées via l'API GitHub (`game-icons/icons`, arbre `master` récursif) avant ajout au
  catalogue, pour éviter tout 404 lors du fetch.
