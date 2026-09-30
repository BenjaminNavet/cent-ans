# GA3-L3 — Figurines de bataille générées (L3a longbowman, L3b 4 unités à pied)

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
- [x] 5. Tests : `ga3_l3_figures_test.gd` (deux modes), smoke, fg3_maps et sr2 (adaptés,
      deux modes), an1a, an1b, fk2, fk_folk, ga1_maps, nt7, nt10, nt12 OK ; capture Godot en
      bataille (1, `--camera=572,595,13,205 --shot-at=4 --no-hud`, 1280×720)
- [x] 6. Docs : `ga3.md` § L3a, ADR 0140 § figurines ; verdict extension (go, L3b)

## Suite (L3b)
- Unités à pied : ajouter une entrée `UNITS` (figure, `top`, `equipment`, et `METAL_Z` par unité
  pour le harnois) et un prompt `UNITS` dans `ga3_fal_figure.py` ; `ga3_figures.py` exporte une
  figure par appel (le manifeste est complété, pas écrasé).
- `equipment()` ne sait construire que des armes de `battle_fine_weapons` sans kwargs : étendre
  pour arbalète + carreaux, pavois dans le dos (`hide_pavise`), armes d'hast.
- Cavalier : rig `cavalry` (os `R:`), alias `cavalry_alias`, cheval fin FG4 gardé.

## Journal
- 30/09 : planche NB2 A-pose correcte en face et dos (profil : mains sur le carquois). trellis-2 :
  99,5 k tri, heat en échec, décimation calée → chaîne voxel + cuisson. Comparatif rendu
  (planche) : les trois modèles se valent à distance de jeu ; multi face+dos garde un dos
  cohérent (le plus vu en bataille), visage un peu plus sombre ; trellis seul : visage plus net,
  dos inventé ; trellis-2 : pas meilleur une fois remaillé à 11,5 k. Livré : multi.
- 30/09 : capture en bataille : archers texturés (jaques de livrée rouge avec crasse, chapels,
  arcs et cordes, chausses variées) ; cadrage à côté du régiment, jugement sur le bord.

# L3b — 4 unités à pied (en cours)
Budget ≤ 1,50 $ (consigne : `trellis/multi` ou `trellis` seulement, ≤ 3 générations 3D par
unité). Figurines retenues (une par type ; `data/unit_types` `figure` + `battle_meshes.VARIANTS`) :
- `man_at_arms` → `infantry_0` (hommes d'armes à pied ; épée + écu) ; mêmes types : infantry_7, 8.
- `crossbowman` → `archer_2` (Génois, pavois : la référence SR3 est génoise) ; archer_1, archer_4.
- `sergeant` → `infantry_1` (piquiers flamands, pique) ; infantry_4.
- `militia` → `infantry_5` (goedendag, Brabançons) ; infantry_2 (milice urbaine), 3, 6.

## Étapes L3b
- [x] 1. Prompts `ga3_fal_figure.py`, `UNITS` de `ga3_figures.py` en données (équipement tiré de
      la recette fine, `metal_z`, clips), `equipment()` générique (registre fin puis V2 ; faces
      `C_ARMS` gardent l'UV d'armoiries), shader `GA3_TEX` épargne `C_ARMS`, planche `board`
- [x] 2. Générations (planche NB2 + bria + multi face/dos) des 4 unités : 1 essai chacune,
      0,632 $ (planches propres, A-pose, mains vides ; épée au fourreau de l'homme d'armes gardée)
- [x] 3. Blender : 4 figurines (heat 100 %), rendus aux poses du jeu (LOD0 exporté skinné CPU
      avec la texture d'os, comme la figurine actuelle), planche `docs/img/ga3/l3b_units.jpg` ;
      îlots voxel < 2 % seulement retirés (jambes sous le tabard de l'arbalétrier = îlots)
- [x] 4. Tests : `ga3_l3_figures_test.gd` (5 figurines, deux modes), an1a (étendu aux figurines
      générées, deux modes), an1b, sr2 (saute les figurines générées), fg3_maps, nt7, nt10, nt12,
      fk2, fk_folk, smoke, pytest `test_ga3_figures_manifest.py` : OK. Capture Godot en bataille
      (1, `--closeup --closeup-distance=14 --no-hud`) : cadrée sur la cavalerie française, aucune
      figurine à pied visible — jugement sur la planche (poses du jeu).
- [x] 5. Docs `ga3.md` § L3b, ADR 0140 § extension L3b, budget (0,63 $)

## Correctifs L3b (valent aussi pour archer_0, reconstruit)
- Fuites de poids « heat » des mains (doigts aliasés en `Wrist`) vers les cuisses et tibias :
  `strip_arm_leaks` retire tout poids de bras sous 0,6 m (après la chaleur et en pose de liaison)
  et renormalise ; sans cela le mouvement secondaire (AN1a) et les bras déplaçaient les chausses.
- Faces de livrée atteignant les tibias (< 0,5 m) rendues à la couleur générée.
- Seuil des îlots voxel 30 % → 2 %.

## Suite
- L3c : cavalier du chevalier. Autres recettes des mêmes types si validé par le joueur.
- Capture en bataille ciblant les fantassins (Crécy : `--historical=crecy`, hommes d'armes,
  Génois, archers) à faire par la session principale.
