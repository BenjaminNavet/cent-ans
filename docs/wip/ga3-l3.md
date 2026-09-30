# GA3-L3a — Figurine de bataille générée : longbowman (archer_0)

Branche `feat/ga3`, worktree `../game_project-ga3`. Budget L3a ≤ 2 $ : dépensé 0,48 $
(`docs/budget.md`). Brutes : `~/dev/cent-ans-raw/ga3/l3/longbowman/` (planche, découpes,
`multi_front_back.glb`, `trellis.glb`, `trellis2.glb`, rendus `renders_<modèle>/`, `.blend`).
Contexte : `docs/wip/ga3.md` (S2, S3, § L3a), ADR 0140 § figurines.

## Consigne de reprise
> Lis ce fichier et `git log --oneline -10`, continue à la première case non cochée.

## Chaîne retenue (décidée)
1. `tools/experiments/ga3_fal_figure.py RAW --unit longbowman [--model multi|trellis|trellis2]` :
   `nano-banana-2/edit` 2K depuis `sr3/longbowman.png` (A-pose, mains vides, 3 vues ; **couleurs
   clés** : jaque vert saturé = livrée, chausses bleu saturé = étoffe) → `bria` sur la planche →
   découpe des 3 vues → **`fal-ai/trellis/multi` face + dos** (0,02 $). Consigne du joueur (via
   le coordinateur) : rester sur le moins cher, pas de trellis-2 ni Meshy ; l'appel trellis-2
   déjà parti est gardé pour comparaison seulement. Le profil généré tient le carquois : exclu.
2. `blender -b --factory-startup --python tools/blender_scripts/ga3_figures.py -- longbowman
   [--renders] [--glb longbowman/X.glb]` : nettoyage, mise à l'échelle (chapel à 1,80 m),
   **solidify 12 mm + remaillage voxel 8 mm** (les surfaces TRELLIS sont des feuilles ouvertes
   doublées : chaleur des os en échec total, décimation « collapse » calée), réduction à
   11 490 tri, UV intelligente, **cuisson Cycles** (albédo étalonné + classes teintables),
   poids « heat » sur rig de segments (chaîne S2), retour en pose de liaison, LOD1/LOD2 par
   décimation (poids et UV gardés), arc/corde/flèche `battle_fine_weapons.longbow`, export `CAM1`.
3. Planche : `uv run --with pillow python tools/blender_scripts/ga3_figures.py sheet longbowman
   docs/img/ga3/l3a_archer.jpg --tags multi_front_back,trellis,trellis2`.

## Jeu
- `game/assets/models/battle_ga3/` : `archer_0_lod{0,1,2}.mesh.bin` (11 800 / 1 350 / 260 tri,
  = plafonds `TRI_CAP` des fantassins fins), `archer_0_albedo.png` 1024² (RGB albédo, A masque
  teintable ; import BPTC, mipmaps, `fix_alpha_border=false`), `manifest.json`.
- `BattleSkinned._merge_ga3` (après `_merge_fine`) : LOD, triangles, variantes (1), albédo,
  luminances ; retire `atlas_layer` (pas de FG3/SR2 sur cette figurine). `_setup_ga3` : variante
  `GA3_TEX` (+ `BV2_CORPSE` pour les cadavres ; `corpse_shader(kind, variant)` ; `folk_pool` garde
  le define). `--no-ga3-fig` après `--` = figurine fine d'origine.
- Shader `GA3_TEX` : UV ≥ 0 → albédo ; `C_LIVERY` : livrée/armoiries/croix du jeu × luminance
  texture / moyenne ; `C_CLOTH` : couleur naturelle par soldat (PLAIN) × idem ; `C_PLATE` (chapel)
  : albédo + métal ; ailleurs l'albédo. Équipement en u < 0 : rendu inchangé.

## Étapes
- [x] 1. Squelette (script fal, note) — commit
- [x] 2. Référence NB2 + 3D (trellis-2 d'abord, puis trellis/multi et trellis sur consigne)
- [x] 3. `ga3_figures.py` : voxel + cuisson, rig, LOD0/1/2, masque, export CAM1 + albédo + manifeste
- [x] 4. Jeu : fusion du manifeste, variante `GA3_TEX`, `--no-ga3-fig`
- [ ] 5. Tests (`ga3_l3_figures_test.gd` OK dans les deux modes ; smoke, fg3/sr/fk/nt en cours),
      capture Godot en bataille (1)
- [ ] 6. Docs : `ga3.md` § L3a, ADR 0140 § figurines ; verdict extension

## Journal
- 30/09 : planche NB2 A-pose correcte en face et dos (profil : mains sur le carquois). trellis-2 :
  99,5 k tri, heat en échec, décimation calée → chaîne voxel + cuisson. Comparatif rendu
  (planche) : les trois modèles se valent à distance de jeu ; multi face+dos garde un dos
  cohérent (le plus vu en bataille), visage un peu plus sombre ; trellis seul : visage plus net,
  dos inventé ; trellis-2 : pas meilleur une fois remaillé à 11,5 k. Livré : multi.
