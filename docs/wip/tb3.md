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

## Troisième passe : jugement visuel sur captures (02/10, nuit ; 16 lectures sur 30)
- [x] Maquettes redessinées en signes en volume (`tb3_outbuildings.py`) : pièces grosses et
      hautes, une silhouette par famille ; 24 maquettes + chantier, 104 à 1 555 triangles.
- [x] Signes de colonie (`sign_village`, `sign_town`, `sign_walled`, `sign_city`, `sign_castle`,
      `sign_suburb`) tenus à l'écran par-dessus la ville 1:1 (`screen.signs`, `suburb_sign`).
- [x] Pose par pièce sur le relief (`pieces` du manifeste, `with_pieces`, `drape` du shader).
- [x] Volumes relevés (× 1,3), teintes éclaircies, façade vers la caméra ; port à la côte.
- [x] `game/tests/tb3_catalogue_shot.gd` (planche catalogue) ; `tb3_shot.gd` pleine résolution,
      planche par ville, troisième ville côtière (La Rochelle : port et saline de niveau 3).
- Planches finales (hors dépôt) : `~/.cache/cent_ans/tb/tb3/tb3-catalogue.png`,
  `tb3-catalogue-z2.png`, `tb3-planche-set_agen.png`, `tb3-planche-set_fleurance.png`,
  `tb3-planche-set_la_rochelle.png` (+ captures entières `tb3-<id>-<d>-avant|apres.png`).

## Mesures (02/10, nuit)
- `tb3_growth_test` OK (6 étapes) : largeurs autour du point visé, en 900 px de haut : 34,5 à
  62,1 px à 20 ; 29,7 à 62,2 px à 45 ; 29,6 à 64,2 px à 90 ; aucune paire en recouvrement ;
  6 maquettes sur 6 autour d'Agen ; sous brouillard, la ville ne garde que son signe.
- `smoke`, `settlements_render_test`, `tb1_seasons_test`, `sz4b_colonies_forests_test`,
  `tf_far_shader_test` OK ; pytest du schéma 11/11. `tb2_declutter_test` échoue (étiquettes
  coupées au bord) avec et sans `--no-tb3` : pas ce lot.
- Appels de dessin (`tb3_shot.gd --bench`, Agen), couche masquée → affichée : d = 20 : 642 → 692 ;
  d = 45 : 449 → 528 ; d = 90 : 478 → 506 ; d = 400 : 398 → 398. Temps par image inchangé
  (16,7 ms, synchro verticale). Mise en place d'un voisinage : 0,4 à 0,9 s étalées.

## Points ouverts (jugement après lecture des planches)
- Mine : la moins lisible des huit familles (terril gris, bouche noire, tour à roue) ; se
  confond de loin avec un four. Vue seulement en catalogue, pas en situation.
- Murs des bâtiments du kit bruns plutôt que clairs (textures de l'atlas) : les toits rouges et
  les étals portent la lecture, pas les murs.
- Signe de cité dominé par l'ardoise grise ; ville saccagée (suie) très sombre.
- Grands volumes : un clocher peut masquer la maquette derrière lui (pas de recouvrement au sol,
  mais à l'écran en plongée).
- Port tourné vers l'eau à ± 70° de la caméra au plus : jetées pas toujours dans l'axe de l'eau.
- Largeur tenue d'après la distance du rig : fond de l'image plus petit (perspective).
- « Échelle réelle sous 15 » : réelle sous 10, fondu de 10 à 15.
- Régions denses : maquettes sans place retirées au palier ; villages, châteaux et abbayes n'ont
  plus que leur signe au-delà de 60 (`minor_until`).
- Signe de colonie = second écart à l'ADR 0138 (ADR 0162) ; `signs: {}` le retire.
- Suie d'une prise : mémoire de session seulement. Règles de niveau : à juger en partie pilote.
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
