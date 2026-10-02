# TB3 — colonies et bâtiments qui poussent

Branche `feat/tb3`, worktree `../gp-tb3`. Plan : `docs/design/2026-10-02-campagne-tob.md` § 3.
Sans service payant (ADR 0152). ADR du lot : `docs/decisions/0162-…` (choix des maquettes, taille tenue à l'écran).

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
- [x] ADR `docs/decisions/0162-batiments-hors-les-murs-assembles-du-kit.md`

## Retouches après lecture des captures (02/10, soir)
- [x] 1. Taille tenue à l'écran (`render.screen`) : échelle réelle sous 10, largeur d'écran tenue à
      partir de 15 (4,2 / 5,8 / 7,7 % de la hauteur d'écran), fondu 120 → 160 ; mise en place de
      loin sans recouvrement (`_layout`, `_far_spot`), hors mer et fleuves, routes évitées ;
      faubourgs et enceinte ajoutés grossis de même. ADR renuméroté 0162 et amendé.
- [x] 2. Brouillard de guerre : rien dans une province hors de vue (`fog_source`, `is_hidden`).
- [x] 3. Suie bornée : toits brun-noir à 60 % au plus, murs à 15 % (deux shaders).
- [x] 4. `tb3_shot.gd` : distances 20, 45, 90 ; paire `-avant` / `-apres` par cadrage.
- [x] `tb3_growth_test` : 6 étapes (largeurs projetées, recouvrements, brouillard, suie bornée).

## Mesures (02/10, machine chargée à 140)
- `tb3_growth_test` OK (6 étapes). Largeurs autour du point visé, en 900 px de haut : 28,8 à
  60,1 px à 20 ; 32,4 à 67,1 px à 45 ; 33,1 à 65,8 px à 90 ; niveau 3 = 1,83 × niveau 1 ; aucune
  paire en recouvrement (30, 60 et 27 maquettes posées) ; 6 maquettes sur 6 autour d'Agen.
- Appels de dessin (`tb3_shot.gd --bench`), couche masquée → affichée : d = 20 : 643 → 679 ;
  d = 45 : 449 → 490 ; d = 90 : 478 → 499 ; d = 400 : 398 → 398. Temps par image : dans le bruit.
- Mise en place d'un voisinage : 0,2 à 0,6 s réelles par tranches de 1,5 ms (9 à 64 colonies).
- pytest complet (avant les retouches) : 1489 réussis, 5 échecs étrangers au lot
  (`test_entity_icons` ×2, `test_ink_icons`, `test_relief_update`, `test_water_detail`).

## Points ouverts
- Largeur tenue d'après la distance du rig : le fond de l'image est plus petit (perspective) ;
  28 px garantis autour du point visé seulement.
- « Échelle réelle sous 15 » de la demande : réelle sous 10, fondu de 10 à 15 (sinon saut de
  taille à 15). Réglable (`real_below`, `full_from`).
- Régions denses : les maquettes sans place manquent au palier ; villages, châteaux et abbayes
  n'en ont plus au-delà de 60 (`minor_until`).
- D'un palier de distance à l'autre, une maquette peut changer d'angle autour de sa ville.
- Maquettes grossies posées à l'altitude de leur centre : sur relief marqué, un bord peut
  flotter ou s'enfoncer. À juger sur capture.
- Suie d'une prise (saccage, assaut) : en mémoire de session seulement (pas d'état dans `core/`).
- Règles de niveau (fermes, salines, mines sans bâtiment propre) : à juger en partie pilote.
- Coût CPU de la mise en place à mesurer sur machine calme.

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
faubourgs, enceinte de pierre, suie ; distances 20, 45, 90 ; `-avant` / `-apres`). Puis fusion
dans `feat/tb`.
