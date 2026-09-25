# Lot V2 — Soldats et chevaux animés (audit A1-18, A1-07, A1-01)

Branche `worktree-agent-aa60537d86d664cc6` (a fusionné `main` puis `integration/night` pour les
assets D0 : Quaternius dans `game/assets/third_party/`). ADR : `docs/decisions/0014-figurines-skinnees-texture-os.md`.

## État : terminé (non fusionné)
Figurines skinnées avec animations cuites en texture d'os, 9 figurines (6 à pied, 3 montées),
intégrées au rendu de bataille, captures avant/après, banc A/B, smoke vert. Aucune dépense
cloud (textures procédurales) : rien à consigner dans `docs/budget.md`.

## Approche
- Pipeline Blender `tools/blender_scripts/battle_skinned.py` (+ `_figures` recettes, `_equipment`
  casques et utilitaires, `_weapons` armes/écus/tabard, `_poses` surcouches de poses et IK,
  `_cavalry` cheval + cavalier) : `blender -b --python tools/blender_scripts/battle_skinned.py`
  (≈ 1 min 30, tout, sortie déterministe) ; `-- --no-rigs --only cavalry_0` pour ne refaire
  qu'un maillage (si `grip()` ou les os virtuels changent, tout refaire).
- Sortie `game/assets/models/battle_skinned/` : `human.bones.bin`, `cavalry.bones.bin`,
  `<figure>_lod{0,1,2}.mesh.bin`, `manifest.json`, `SOURCE.md` (formats).
- Godot : `game/scripts/battle/battle_skinned.gd` (chargeur, matériau, `STYLES` états → clips),
  `game/shaders/battle_soldier_skinned.gdshader`, intégration dans `battle_soldiers.gd`
  (`_skinned`, `_make_skinned_material`, `SKINNED_DETAIL_DISTANCE`, cadavres en mode CUSTOM).
  Repli `--rigid-figures` (A/B).
- Test visuel hors simulation : `game/tests/v2_figures_shot.gd` (`--fig=`, `--state=`, `--time=`,
  `--since=`, `--cam=`, `--rigid`).

## Figurines (triangles LOD0 / LOD1 / LOD2)
| Figurine | Contenu | Triangles |
|---|---|---|
| `infantry_0` hommes d'armes | bassinet + camail, jupon de livrée, mailles, plates, épée ; écu armorié (1 sur 2) | 2 374 / 517 / 255 |
| `infantry_1` piquiers flamands | gambison, chapel / bassinet / bonnet, pique à deux mains | 2 200 / 505 / 229 |
| `infantry_2` milice et paysans | tabard de livrée, bonnet / chapel / tête nue, lance ou vouge ; paysans (chapeau de paille, fourche) | 2 231 / 561 / 277 |
| `archer_0` archers anglais | veste de livrée, chapel / feutre / tête nue, arc long, carquois | 2 094 / 503 / 205 |
| `archer_1` arbalétriers | gambison, chapel / bassinet, arbalète, étui à carreaux | 2 220 / 518 / 222 |
| `archer_2` génois | bassinet + camail / chapel, pavois armorié au dos | 2 360 / 562 / 258 |
| `cavalry_0` chevaliers | heaume ou bassinet, écu, lance à pennon, caparaçon armorié, 5 robes | 3 052 / 800 / 426 |
| `cavalry_1` sergents montés | gambison, chapel / bassinet, lance | 2 747 / 693 / 341 |
| `cavalry_2` archers montés | arc long, carquois | 2 549 / 682 / 310 |

Clips : `human` (idle, guard, walk, run, slash, thrust, hit, death, death_m, death_knees,
death_back, knockdown, pike_idle/walk/level/level_walk/thrust, bow_shoot/idle/walk,
xbow_shoot/idle/walk) ; `cavalry` (c_idle, c_walk, c_gallop, c_charge, c_thrust,
c_bow_idle/walk/shoot, c_death, c_death_m, c_fall).

## Mesures (M4 Pro partagé, `--disable-vsync --resolution 1600x900 -- --benchmark --bench-at=90`,
A/B alterné `--rigid-figures` / skinné, moyennes i/s sur 600 images ; écran plafonné à 60 Hz)
| Config | rigide (B1/B4) | skinné (V2) | Primitives |
|---|---|---|---|
| `--units=20` (4 580 soldats), vue lointaine | 58,6 · 54,0 (56,3) | 57,5 · 51,8 (54,7) | 2,87 M → 2,34 M |
| `--units=20`, `--camera=630,240,28,180` (gros plan) | 40,0 · 54,7 (47,4) | 49,2 · 54,7 (52,0) | 3,70-3,93 M → 3,12-3,23 M |
| `--units=50` (11 800 soldats), vue lointaine | 47,4 · 54,9 · 41,9 (48,1) | 51,7 · 57,3 · 57,5 (55,5) | 2,50 M → 2,09 M |
| `--units=50`, gros plan | 42,6 · 40,7 · 29,1 (37,5) | 37,7 · 37,3 · 39,1 (38,0) | 3,79 M → 3,17 M |
Avant les optimisations (4 os partout, interpolation partout, configurations recalculées à
chaque image) : `--units=50` gros plan 34,7 contre 43,6 (−20 %). Objectif « pas pire que
−15 % » tenu ; le bruit de la machine (±25 %) domine les écarts restants.

## Captures (`docs/audit/captures/v2/`, `avant_` = `--rigid-figures`, même graine, même instant)
`bataille_charge_gros_plan` (`--closeup`), `bataille_cavaliers` (`--shot-at=100
--camera=600,400,30,35`), `bataille_melee`, `bataille_vue_large` (A1-01 : pas de régression de
lisibilité), `figurines_{archers,arbaletriers,melee,piques,charge,cavaliers_melee,morts}`.

## Pièges rencontrés (à garder en tête)
- Import glTF : `guess_original_bind_pose=False` indispensable (sinon maillage de repos décalé
  de plusieurs mètres) ; la pose de repos est alors bras le long du corps (pas un T).
- Les axes des os importés ne suivent pas les membres (poignet orienté vers le bas…) : l'IK vise
  la tête de l'os enfant (`LIMB_CHILD`), la main prolonge l'avant-bras.
- Remettre `matrix_basis` à l'identité avant chaque `frame_set` (canaux non animés sinon
  pollués par la surcouche de l'image précédente).
- `Matrix.to_quaternion()` sur une matrice mise à l'échelle (armature ×97) : normaliser d'abord.
- Couleurs de sommets en 8 bits dans Godot : code matière en alpha / 16.
- `Foot.*` des personnages : enfants de `Root`, pas du tibia (déplacer à la main après IK).
- Pièces recréées sur une armature déplacée (cavalier en selle) : suivre l'empty racine
  (`create_part`).

## Demandes du coordinateur (points d'extension, cf. ADR 0014)
Morts multiples (fait : `death`, `death_m`, `death_knees`, `death_back`, `c_death`, `c_death_m`,
`c_fall`, `knockdown`), canal sang par instance (`INSTANCE_CUSTOM.w` + uniforme `blood`, masque
prêt, inactif à 0), cadavre figé (mode CUSTOM, instant −1000), LOD1/LOD2 + piste d'imposteurs.

## Points ouverts
- Glissement des pieds : la cadence de marche/course n'est pas calée sur la vitesse de la
  simulation (`anim_speed` par style seulement).
- Piques abaissées : bras un peu tendus ; arme d'hast de la milice tenue d'une main (suit le
  poignet) ; épée tenue horizontale en marche.
- Pas de duels appariés ni de réaction aux coups synchronisée (A1-20), pas de rênes ni de
  bride ; caparaçon rigide (traversé par les jambes au galop).
- Engins de siège et leurs servants : toujours procéduraux (B1/B4).
- Imposteurs lointains : décrits, non faits.
