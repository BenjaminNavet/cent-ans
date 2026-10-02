# FA3 — animations de combat CC0 reciblées sur les figurines fines

Branche `feat/fa-anim` (worktree `../gp-fa-anim`, issue de `feat/fa`). Chantier : `docs/wip/fa.md`.
Prédécesseurs : NT12 (`docs/wip/nt12-mocap.md`), NT13/NT14 (`docs/wip/nt13-video-mocap.md`).

## État : squelette (10-02)
- Table des clips : `data/fx/fa3_anim_sources.json` (schéma `fx_fa3_anim_sources.schema.json`,
  test `tools/tests/test_fa3_anim_sources_schema.py`).
- Script `tools/blender_scripts/fa3_anim_retarget.py` : vide.

## Sources (hors dépôt, `~/dev/cent-ans-raw/fa/anim/`)
- Mesh2Motion (CC0), `m2m-app-glb/human-{base,addon,mocap}-animations.glb`, 66 os, pose en T.
- KayKit Character Animations 1.1 (CC0), `Rig_Medium_CombatMelee.glb`, 23 os.

## Constats sur les sources (sondage Blender)
- Les deux squelettes regardent -Y après import glTF, pose de repos en T.
- Clips Mesh2Motion à coup unique : la dernière image répète la première (clé de bouclage) :
  `last: -2` dans la table.
- `Sword_Regular_A/B/C` et `_Rec` avancent de 0,8 à 1,1 m (os `root` animé) : inutilisables
  sur place ; `Sword_Attack` (coupe large, sur place) sert pour `slash`.
- Certaines actions ne posent pas toutes les pistes : remettre la pose à zéro avant chaque image.

## Prochaine étape
Écrire le reciblage (lecture GLB, rotations, IK des pieds, cuisson `CAB1`), puis le drapeau
`--fa-anim`, le test Godot et les planches.
