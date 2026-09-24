# Lot V2 — Soldats et chevaux animés (audit A1-18, A1-07, A1-01)

Branche `worktree-agent-aa60537d86d664cc6` (a fusionné `main` puis `integration/night` pour les
assets D0 : Quaternius dans `game/assets/third_party/`). ADR : `docs/decisions/0014-figurines-skinnees-texture-os.md`.

## Approche
- Pipeline Blender `tools/blender_scripts/battle_skinned.py` (+ `_figures` recettes, `_equipment`
  casques et utilitaires, `_weapons` armes/écus/tabard, `_poses` surcouches de poses et IK,
  `_cavalry` cheval + cavalier) : `blender -b --python tools/blender_scripts/battle_skinned.py`
  (≈ 1 min 30, tout) ; `-- --no-rigs --only cavalry_0` pour ne refaire qu'un maillage (attention :
  si `grip()`/os virtuels changent, tout refaire).
- Sortie `game/assets/models/battle_skinned/` : `human.bones.bin`, `cavalry.bones.bin` (textures
  d'os), `<figure>_lod{0,1,2}.mesh.bin`, `manifest.json`, `SOURCE.md` (formats).
- Godot : `game/scripts/battle/battle_skinned.gd` (chargeur, matériau, états → clips),
  `game/shaders/battle_soldier_skinned.gdshader`, intégration dans `battle_soldiers.gd`
  (`_skinned`, `_make_skinned_material`, cadavres en mode CUSTOM). Repli `--rigid-figures`.
- Test visuel hors simulation : `game/tests/v2_figures_shot.gd` (`--fig=`, `--state=`, `--time=`,
  `--since=`, `--cam=`, `--rigid`).

## Pièges rencontrés (à garder en tête)
- Import glTF : `guess_original_bind_pose=False` indispensable (sinon maillage de repos décalé
  de plusieurs mètres) ; la pose de repos est alors bras le long du corps (pas un T).
- Les axes des os importés ne suivent pas les membres (poignet orienté vers le bas…) : l'IK vise
  la tête de l'os enfant (`LIMB_CHILD`), la main prolonge l'avant-bras.
- Remettre `matrix_basis` à l'identité avant chaque `frame_set` (canaux non animés sinon
  pollués par la surcouche de l'image précédente).
- `Matrix.to_quaternion()` sur une matrice mise à l'échelle (armature ×97) : normaliser d'abord.
- Couleurs de sommets en 8 bits dans Godot : code matière en alpha / 16.
- `foot` des personnages : enfants de `Root`, pas du tibia (déplacer à la main après IK).

## État
- [x] Rig humain (25 os dont 3 virtuels, 23 clips) et rig monté (70 os, 11 clips).
- [x] 9 figurines : homme d'armes (bassinet + camail, épée, écu armorié 1/2), piquiers flamands,
  milice et paysans (4 variantes : bonnet/chapel/tête nue/chapeau de paille ; lance, vouge,
  fourche ; tabard de livrée), archers anglais (arc long tiré à la joue), arbalétriers, génois
  (pavois), chevaliers (heaume ou bassinet, écu, lance à pennon, caparaçon armorié), sergents
  montés, archers montés. LOD0 2 100-3 050, LOD1 500-800, LOD2 205-430 triangles.
- [x] Shader (texture d'os, interpolation, fondu d'état, variantes, matières procédurales,
  livrée éclairée au loin) + intégration bataille (vérifiée : `--closeup`).
- [ ] Banc A/B (en cours) ; captures `docs/audit/captures/v2/` ; smoke ; ajustements visuels.

## Demandes du coordinateur (points d'extension, cf. ADR 0014)
Morts multiples (fait : `death`, `death_m`, `death_knees`, `death_back`, `c_death`, `c_death_m`,
`c_fall`, `knockdown`), canal sang par instance (`INSTANCE_CUSTOM.w` + uniforme `blood`, masque
prêt), cadavre figé (mode CUSTOM, instant −1000), LOD1/LOD2 + piste d'imposteurs (décrite).

## Prochaine étape
Mesures A/B, captures avant/après, smoke, puis finitions (variété, visages).
