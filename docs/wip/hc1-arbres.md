# HC1 — arbres généralisés sur la carte de campagne

Worktree `../gp-hc1`, branche `feat/hc1` (issue de `feat/hc`). ADR 0161. Aucun changement Rust.

## État
- [x] Squelette : `map.tree_style` (JSON + schéma), `MapPropScale.tree_style()` /
      `--tree-style=`, champs `generalised_*` du `.tres`, test, cette note.
- [x] Rendu généralisé dans `vegetation.gd` (écrit, à valider par les tests) : échelle
      `campaign_prop_scale` = `generalised_tree_height / generalised_reference_height`, pas
      `generalised_spacing`, portée `generalised_max_distance`, disque de dessin centré en avant
      du point visé, paliers et ombres par distance à la caméra, forêt dense et cartes éteintes.
- [x] Dégagements `tree_clearance.gd` : houppiers hors fleuves, lacs, mer, routes principales
      (filtre des tampons après le semis natif, en tâche de fond).
- [x] Test `game/tests/hc_forest_test.gd`, planche `game/tests/hc_shots.gd` (écrits).
- [ ] Faire passer import, test, smoke, gc_maquettes_test.
- [ ] Planches (3 lectures au plus) et réglage des gains hors forêt.
- [ ] Mesures Paris rig 90 / 300 / 700 (`hc_shots.gd --bench`).
- [ ] Tests existants qui supposent le style 1:1 par défaut (vt3, sz4b, settlements_render…).

## Choix
- **Haies** (consigne GC, 02/10) : en style généralisé, pas de buissons alignés sur la trame du
  parcellaire (`VegetationFields`), que GC réduit à des enclos d'environ 1,4 px (`field_scale`) :
  des arbres d'environ 0,8 unité ne peuvent pas les dessiner. Les emplacements `Kind.HEDGE` du
  semis sont vidés (`VegetationTileJob.drop_hedges`) ; le bocage se lit par des arbres épars plus
  nombreux (rôle « isolé », poids `generalised_hedge_boost` sur le canal de haies du masque) et
  les rares arbres de haie du semis (2,5 % des points de haie). Style `real` inchangé.
- **Emprises des lieux** : prises telles que `SettlementLayer.vegetation_exclusions()` les renvoie,
  élargies du rayon nominal d'un houppier (`MapPropScale.generalised_crown_radius()`), sans rayon
  en dur.
- **Taille** : hauteur monde en réglage `.tres` (`generalised_tree_height`, départ 0,7), à tenir
  nettement sous la largeur d'un village (3,2 unités, GC2).
- Aucune donnée ni règle touchée ; `tree_species.json` (compilé) non modifié : les probabilités
  hors forêt sont multipliées par les gains `generalised_*_gain` du `.tres`.

## Prochaine étape
Attendre l'import, lancer `hc_forest_test.gd`, corriger, puis première planche.

## Points ouverts
- Éclaircissement au dézoom (`generalised_far_density`) : à décider après mesure.
