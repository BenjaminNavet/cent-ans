# L3 — Villes emblématiques : matériaux et sièges

Branche `worktree-agent-af2974d71a33b3dc3`. Suite de L1 (`docs/wip/l1-paris.md`) et L2
(`docs/wip/l2-villes.md`), ADR 0015 et **ADR 0026** (sièges dans le plan). Captures :
`docs/audit/captures/l3/`.

## État : terminé (à fusionner par l'orchestrateur ; `main` fusionné, SG1 compris)

- [x] Atlas partagé `game/assets/textures/landmarks/` (Texture2DArray 16 couches 512², albédo
  « détail » + normales ; Poly Haven CC0 + couches procédurales : pans de bois, plomb, vitrail, sol,
  masques de vieillissement), `build_textures.py`, `SOURCE.md`, `CREDITS.md`. Réglages globaux
  `shader_globals` (project.godot) : aucun script qui pose le shader n'a dû changer (menu MM1 compris).
- [x] `landmark.gdshader` : triplanaire local, couleur de palette × détail, variation par bâtiment
  (hachage de l'ancrage), grandes nuances, pied des murs, coulures, plaques, mousse sur les faces au
  ciel ; normales plates dérivées (plus de NORMAL dans les GLB) ; `meters_per_unit`, `ground_height`,
  `debug_view` (1 hauteur, 2 normale, 3 masques, 4 bruit, 5 hauteur/exagération/couche).
- [x] Générateur : alpha de la teinte = couche × 16 + pas d'exagération (`material_code` : une pierre
  garde sa taille sur Notre-Dame agrandie ×2,6), bloc `materials` par ville (murs/toits des maisons,
  palette : calcaire blond, Caen, pierre d'Avignon, brique de Bruges), `drop_normals`.
  Poids : Paris 5,7 Mo (était 10), paris_siege 5,3 (9,4), Londres 4,0, Avignon 3,3, Calais 1,7,
  Rouen 4,0, Bordeaux 3,8, Bruges 3,9 ; toiles de fond 0,03-5,7 Mo.
- [x] Toiles de fond des 6 villes L2 (bloc `siege` : `cut_m`, `center_x_m` nouveaux), hauteurs cuites
  au maximum du texel dans `LandmarkBackdrop`, toile de fond choisie par le `siege_layout` du cœur.
- [x] Siège dans le plan (ADR 0026) : `data-model` `Landmark` (`GameData.landmarks`),
  `sim-battle/src/siege_layout.rs` (`SiegeLayout::from_landmark`, `SiegeWorks::from_layout` /
  `for_battle`, repli générique), `BattleSetup.siege_layout` joint par `battle_request.rs`, pont
  `BattleSim.get_siege_landmark()`, `LandmarkSiegeTown` (rues pavées), titre « Assaut de Paris (Porte
  Saint-Jacques) », démo `--siege-province=<prov>`. Tests `sim-battle/tests/l3.rs` (4) + unitaires (4),
  pytest `test_landmark_siege_battle_references`, `test_landmark_model_material_codes`,
  `test_material_code_round_trip`.
- [x] Captures : `avant/apres_menu_cite`, `avant/apres_notre_dame_gros_plan`, `apres_notre_dame_cite`,
  `apres_campagne_paris_d7` (avant : `l1/apres_notre_dame_d7`), `apres_campagne_bruges_d7`,
  `apres_campagne_avignon_d7`, `apres_siege_paris` (avant : `l1/apres_siege_paris`),
  `siege_paris_assaut` (avec SG1), `siege_london`, `siege_rouen`, `siege_bordeaux`.

## Points ouverts
- Avignon et Bruges : pas de guerre au départ, la démo ne peut pas les assiéger (géométrie testée en Rust).
- Toiles de fond de Bordeaux et Calais maigres (La Bastide vide au XIVe ; pas de mer dessinée à Calais).
- Occlusion ambiante non cuite dans Blender (vieillissement procédural seulement) ; salissures et
  coulures discrètes sous le soleil de la campagne.
- Monuments intérieurs non posés dans la ville assiégée (échelle ≈ 1/6) : seule la toile de fond les montre.

## Outils
- Gros plan : `godot --path game --script res://tests/l3_landmark_shot.gd -- --model=paris_siege
  --cam=x,y,z --target=x,y,z --out=<png> [--flat] [--debug=1..5]`.
- Siège d'une ville : `godot --path game res://scenes/battle/battle.tscn -- --siege-province=prov_ile_de_france
  --deploy-shot --camera=600,600,420,180 --screenshot=<png>`.
- Régénérer : `blender --background --python tools/blender_scripts/landmark_city.py --
  data/landmarks/<id>.json game/assets/models/landmarks/<id>[_siege].glb [--siege]`.
- Atlas : `uv run --no-project --with pillow --with numpy python
  game/assets/textures/landmarks/build_textures.py <téléchargements Poly Haven 1k>`.
