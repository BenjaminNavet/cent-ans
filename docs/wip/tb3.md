# TB3 — colonies et bâtiments qui poussent

Branche `feat/tb3`, worktree `../gp-tb3`. Plan : `docs/design/2026-10-02-campagne-tob.md` § 3.
Sans service payant (ADR 0152). ADR du lot : `docs/decisions/0153-…` (choix des maquettes).

## État
- [x] 1. `data/map/building_models.json` + schéma + `tools/tests/test_building_models_schema.py`
- [x] 2. Maquettes : `tools/blender_scripts/tb3_outbuildings.py` → `game/assets/models/outbuildings/`
      (24 maquettes + `worksite_1`, 154 à 2 438 triangles, une surface `Building`)
- [x] 3. `game/scripts/map/outbuilding_layer.gd` (un MultiMesh par maillage pour le voisinage de la
      caméra), branché dans `SettlementLayer` (`outbuildings`)
- [x] 4. `game/scripts/map/town_growth.gd` : faubourgs (population) et enceinte (fortification) ;
      `replace_models` retiré, `growth_of(id)` à la place
- [x] 5. Suie par ville : `town_soot.gd`, paramètre d'instance `town_soot`
      (`town_building.gdshader`), masque `soot_mask` (`town_far.gdshader`)
- [x] 6. Chantier : `construction_markers.gd` (maquette à taille d'écran constante) + chantier 1:1
- [x] Tests : `game/tests/tb3_growth_test.gd` (5 étapes), `game/tests/tb3_shot.gd` (captures, banc)
- [x] ADR `docs/decisions/0153-batiments-hors-les-murs-assembles-du-kit.md`

## Mesures (02/10, machine chargée à 30-50)
- `tb3_growth_test` OK (5 étapes) ; `smoke`, `settlements_render_test`, `sz4b_colonies_forests_test`,
  `tf_far_shader_test`, `tb2_declutter_test`, `tb1_seasons_test` OK ; pytest
  `test_building_models_schema.py` 9 tests OK.
- Appels de dessin (`tb3_shot.gd --bench`) : d = 8 : 696 → 714 ; d = 90 : 478 → 478 ; d = 400 :
  398 → 398. Temps par image : bruit de charge (détail dans `docs/godot-map.md`).
- `ss_shot.gd --bench` avant / après : 20,6 → 22,2 ms à 90 ; 23,7 → 23,7 ms à 400 (bruit).
- pytest complet : `test_entity_icons.py` échoue déjà sans ce lot (miniature de la collégiale).

## Points ouverts
- Échelle réelle : rien de visible à d = 90 ni 400 (ADR 0138). `render.exaggeration.max` dans les
  données si le joueur veut voir les domaines de plus loin.
- Suie d'une prise (saccage, assaut) : en mémoire de session seulement (pas d'état dans `core/`).
- Règles de niveau (fermes, salines, mines sans bâtiment propre) : à juger en partie pilote.
- Brouillard de guerre : les bâtiments des provinces non vues sont dessinés comme les villes 1:1.
- Numéro d'ADR 0153 déjà pris sur main par le lanceur Windows.

## Relevé de départ
- Pont : `get_province_city(id)` donne `buildings[{id, name, category, upkeep}]`,
  `fortification_level`, `resources`, `construction` ; `get_provinces_snapshot` donne
  `population_total`, `devastation`, `besieged`, `constructing`.
- Aucun état « ville saccagée » n'est conservé par `core/` : le saccage ajoute de la dévastation
  à la province (`capture.rs`). La suie par ville se lit donc sur la dévastation et le siège.
- Les bâtiments de `data/buildings/` n'ont pas de niveau propre : ils ont un `tier` et une chaîne
  `upgrades_from`. Le niveau de maquette vient donc de la correspondance.

## Prochaine étape
Relecture visuelle par la session principale :
`godot --path game --resolution 1600x900 --script res://tests/tb3_shot.gd -- --out=<dossier>`
(Agen : ferme, vignoble, marché de niveau 3, moulin et abbaye de niveau 1, chantier ; Fleurance :
faubourgs, enceinte de pierre, suie). Puis fusion dans `feat/tb`.
