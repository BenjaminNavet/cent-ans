# TF — Colombage TimberFrame + toits du château Kenney (lot L4 de realisme-suite)

Branche `feat/tf`, worktree `/Users/jean_hubert/dev/game_project-tf`.

## État
- [x] Lecture : kit Blender, kit_export, building_materials, atlas campagne.
- [x] Captures « avant » (`~/dev/cent-ans-raw/tf/before_{village,castle}.png`) : scripts
      `game/tests/tf_village_shot.gd` (village normand, `--no-speech --no-cinematic` obligatoires :
      le discours d'ouverture reprend la caméra) et `tf_castle_shot.gd` (cité + château seuls).
- [ ] TimberFrame câblé (couche en fin d'atlas, `first_plain`, surface du kit, réexport).
- [ ] Choix régional par données (schéma validé).
- [ ] Toits bleus du château Kenney (campagne).
- [ ] Tests + captures avant/après `docs/img/tf/tf_ab.jpg`.

## Prochaine étape
Atlas (couche 14 = TimberFrame), `first_plain` → plage `plain_first..plain_last`, réexport,
règle régionale (`data/art/building_regions.json`), château Kenney converti à l'atlas.
