# SR — Figurines semi-réalistes (orchestration)

Spec : `docs/superpowers/specs/2026-09-30-sr-semi-realiste-figurines-design.md`.
Branche `feat/sr`, worktree `../game_project-sr`. Brutes hors worktree : `~/dev/cent-ans-raw/`.
Mandat : autonomie totale (joueur, 30/09), NB2 ≈ 20 $ au total, sans variantes.

## Consigne de reprise
> Lis ce fichier et `git log --oneline feat/sr -15`, continue à la première case non cochée.

## Lots
- [x] SR1 matières scannées ambientCG (agent `cent-ans-dev`) — couches 0-7 = scans CC0 entiers
  (normale/rugosité/déplacement du scan), `tile_m` physique (0,15/0,13/0,16/0,32/0,51/0,3/0,3/0,8 m),
  couches 8-11 recopiées ; plate ga_mix 0,6 ; planche `docs/img/sr/sr1_layers.jpg`.
- [x] SR3 planches NB2 (session principale, ≤ 1 $) → liste d'écarts
- [x] SR2 usure et métal (après SR1, même shader)
- [ ] SR3b corrections des recettes Blender + recuisson
- [x] SR5 bâtiments, gains rapides (usure atlas/villes, `building_pbr.gdshader`, `--no-sr5`)
- [ ] SR4 captures A/B, perf, tests, ADR 0136, fusion

## Journal
- 09-30 : spec écrite, worktree créé.

## SR1 — fait (agent)
- Chaîne : `uv run --project tools cent-ans assets materials --out <scratch> --scans --build --layers-sheet docs/img/sr/sr1_layers.jpg`.
- Tests : pytest `test_material_gen.py` + `test_sr1_scans.py` (21), `ga1_maps_test.gd` OK (2,33 Mo), `smoke.gd` OK.
- À juger en jeu (SR4) : échelle des fils (0,6-2 mm, fondus par les mipmaps de loin), normale
  plate/cuir renforcée (x4/x3), rugosité laine/gambeson/bois relevée.

## SR2 — fait (agent)
- Shader (variante FG3_BAKED seulement, < `fine_distance`, fondu sur 60-80 m) : uniformes
  `weathering` (0,6) et `sr2_mud_height` (0,45 ; cavalerie 0,65 : jambes du cheval + ourlet du
  caparaçon), posés par `BattleSkinned._setup_fine_maps` ; `--no-sr2` → 0 (sortie SR1 exacte,
  bloc sauté). Boue des pieds (bord bruité, quantité par soldat), crasse des creux (AO cuite,
  étoffe/cuir), acier vivant (rugosité 0,25-0,6 par taches, usure claire des arêtes par
  courbure écran × bruit fin, rouille brune bas des mailles), teintes passées −15 %.
- Écart : l'usure des arêtes vient de la courbure (dérivées de la normale / position, prises en
  flux uniforme) plutôt que de « bruit × AO inversé » (les creux ne s'usent pas).
- Perf : 0 lecture de texture ajoutée ; ≤ 5 `noise2` par fragment (plates 2+3, mailles 1+3).
- Tests : `sr2_weathering_test.gd` (défaut, `--no-sr2`, `--coarse-figures`), fg3/ga1_maps, smoke OK.
- À juger en SR4 (captures A/B `--no-sr2`) : dosage boue, arêtes, teintes.

## SR5 — fait (agent)
- Include `game/shaders/building_aging.gdshaderinc` (bruit procédural, sans texture ; bruit fin
  ramené à sa moyenne quand un pixel couvre > 0,15-0,8 m) : boue au pied (0,5-1,4 m, y local),
  coulures sous les appuis (colonnes ~2,5 m, appui 1 m + 3 m/étage), crasse, toits assombris et
  moussus. Graine par bâtiment : origine de l'instance + INSTANCE_ID.
- SR5a : `aging` (0,5) dans `building_atlas.gdshader` (maquettes + bataille) et
  `town_building.gdshader` (recopié par `TownBuilder.material`).
- SR5b : `building_pbr.gdshader` remplace le StandardMaterial3D de `_textured` (un matériau par
  matière, même nombre de matériaux) ; `--no-sr5` : StandardMaterial3D et `aging` 0 partout.
  Poids : `SR5_MOSS`/`SR5_GRIME` dans `building_materials.gd`.
- `landmark.gdshader` non refactorisé (masques d'atlas, rendu VH4 inchangé).
- Tests : `sr5_buildings_test.gd` (avec et sans `--no-sr5`), ga5, zg6, vh4, smoke OK.
- À juger en jeu (SR4) : intensité 0,5, hauteur de boue (suppose y local = 0 au pied du kit).

## À faire (hors gains rapides SR5)
- Câblage `TimberFrame` : réexport Blender du kit, indice de couche d'atlas, choix régional.
- Toits bleus du château Kenney à remplacer.
- LOD grossier des maquettes (appels de dessin, chantier FPS carte).
