# Lot V2 — Soldats et chevaux animés (audit A1-18, A1-07, A1-01)

Branche `worktree-agent-aa60537d86d664cc6` (a fusionné `main` puis `integration/night` pour les
assets D0 : Quaternius dans `game/assets/third_party/`).

## Approche
- Pipeline Blender `tools/blender_scripts/battle_skinned.py` (+ `_equipment`, `_poses`, `_figures`) :
  pièces Quaternius CC0 recolorées par codes matière, équipement XIVe procédural, décimation par
  pièce (3 niveaux), export binaire `game/assets/models/battle_skinned/<figure>_lod<k>.mesh.bin`
  (zlib) avec 4 os + poids par sommet ; animations cuites en **texture d'os** (`human.bones.bin` :
  3 texels RGBA32F par os et par image, 24 i/s) ; `manifest.json` (os, clips, fichiers).
- Import glTF : `guess_original_bind_pose=False` indispensable (sinon maillage de repos décalé).

## État
- [x] Squelette : rig humain (22 os, 17 clips dont placeholders arc/arbalète/pique), homme d'armes
  `infantry_0` (bassinet + camail + épée), 3 LOD (2 326 / 493 / 239 triangles).
- [ ] Shader `battle_soldier_skinned.gdshader` + chargeur `battle_skinned.gd` + intégration
- [ ] Autres rôles, variantes, chevaux, poses custom, textures, perf, captures, ADR 0014

## Prochaine étape
Chargeur Godot + shader, homme d'armes de bout en bout.
