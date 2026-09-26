# FG4 — Cheval fin en production

Branche : `feat/fg4-horse` (worktree agent, `main` et FG1 intégrés, non fusionnée dans `main`). Plan : `docs/wip/fg-figurines-fines.md`.
Prototype : `docs/wip/fg0-prototype.md`.

## État : terminé, branché sur FG1
- [x] Déformation : ajustement **articulation par articulation** (`battle_fine_horse.OGA_JOINTS`)
  au lieu du champ RBF de FG0 ; poids par chaleur des os calculés dans la pose naturelle du
  cheval CC0, sur un squelette dont les os relient ses propres articulations ; tête portée par
  une seule transformation rigide (plus de tête tordue) ; paupières ouvertes et yeux agrandis.
- [x] Étriers +6 cm dans `battle_skinned_cavalry.py` (`STIRRUP_HALF_WIDTH = 0.36`), surcharge FG0
  retirée de `battle_fine_proto.py` ; `cavalry.bones.bin` recuit (`human.bones.bin` et
  `manifest.json` identiques à l'octet).
- [x] Harnachement : selle de guerre (panneaux moulés sur le dos, troussequin haut, arçon,
  sangle, poitrail, étrivières et étriers), bride (têtière et montants, muserolle, sous-gorge,
  mors), rênes ; bardes selon les recettes : caparaçon deux pièces (FG0), chanfrein, flançois.
- [x] Trois montures (mêmes os donc même hauteur ; amincies autour des os) : **destrier**
  (`cavalry_0`, `cavalry_3`, `standard_1`), **roncin** (`cavalry_1`, `2`, `4`, `6`), **genet**
  (`cavalry_5`).
- [x] Robes : `C_COAT` × ombrage par sommet (`fg_shade` : AO et relief du pelage CC0, bas des
  jambes et bout du nez plus sombres) × robe du shader tirée par soldat (bai, alezan, noir, gris,
  isabelle) — shader inchangé. Crins en `C_COAT` plus sombre (suivent la robe).
- [x] Faces du corps cachées sous le caparaçon supprimées (ou masquées par variante quand le
  caparaçon n'est porté que par une variante, `cavalry_3`).
- [x] Chaîne LOD en `CAM1` dans `game/assets/models/battle_fine/` via `battle_fine.py figures`.
- [x] Planches `docs/img/fg/fg4_clips.png` (destrier caparaçonné), `fg4_clips_cavalry_1.png`
  (roncin, jambes visibles), `fg4_chevaux.png` (montures, robes, tête, LOD) ; captures en jeu
  `fg4_jeu_charge.jpg`, `fg4_jeu_arret.jpg` et `fg4_jeu_charge_avant.jpg` (actuel).

## Triangles (après branchement dans FG1 : cavalier fin)
| Figurine | Monture | cheval LOD0/1/2 | harnachement LOD0/1/2 | figurine LOD0/1/2 |
|---|---|---|---|---|
| cavalry_0 chevaliers | destrier caparaçonné | 5 510 / 1 087 / 279 | 2 471 / 281 / 116 | 17 920 / 3 227 / 779 |
| cavalry_1 sergents | roncin | 5 716 / 1 128 / 284 | 1 557 / 145 / 78 | 14 287 / 2 626 / 635 |
| cavalry_2 archers montés | roncin | 5 716 / 1 128 / 284 | 1 557 / 145 / 78 | 14 166 / 2 585 / 606 |
| cavalry_3 gendarmes | destrier, chanfrein + flançois ou caparaçon | 5 716 / 1 128 / 284 | 2 769 / 325 / 126 | 15 270 / 2 734 / 671 |
| cavalry_4 écorcheurs | roncin | 5 716 / 1 128 / 284 | 1 557 / 145 / 78 | 14 375 / 2 610 / 625 |
| cavalry_5 jinetes | genet | 5 716 / 1 128 / 284 | 1 556 / 145 / 79 | 14 226 / 2 577 / 631 |
| cavalry_6 hobelars | roncin | 5 716 / 1 128 / 284 | 1 557 / 145 / 78 | 14 343 / 2 592 / 631 |
| standard_1 | destrier caparaçonné | 5 510 / 1 087 / 279 | 2 471 / 281 / 116 | 15 453 / 2 794 / 692 |

Cheval LOD0 : corps 3 700, crinière 2 × 600, queue 2 × 350 (cartes doublées : ni alpha ni double
face dans le shader), yeux 120 ; le LOD2 (ombres) reste ≤ 300. Le reste du LOD2 des montés
(~350-400) est le cavalier FG1. Actuel : cavalry_0 2 956 / 746 / ~370.

## Fusion avec FG1 (faite sur la branche, 15ade06c)
- `main` (FG1 fusionné) intégré ; conflits uniquement sur les binaires `battle_fine/cavalry_*`,
  `standard_1_*` (même nom des deux côtés), régénérés depuis.
- Branchement : `battle_fine_figures.build_figure` (montés) construit la monture sans cheval
  Quaternius puis appelle `battle_fine_cavalry.horse_and_harness` (cheval, harnachement, bardes
  de la recette ; objets marqués `fg4`). `FineMount` (FG1) hérite de `Mount`, donc des étriers à
  0,36 m ; aucune autre modification de `battle_fine.py`.
- `battle_fine.py -- rigs` relancé : `battle_fine/cavalry.bones.bin` recuit (étriers larges ;
  `human.bones.bin` identique), puis `figures --only cavalry_0,…,cavalry_6,standard_1`.
  `battle_skinned/cavalry.bones.bin` (rendu par défaut) recuit aussi avec les étriers larges.
- Vérifié : `battle_fine_check.py` (données cuites, skinning CPU : pieds dans les étriers,
  cavalier assis, galop), capture en jeu `--fine-figures` (`tests/v2_figures_shot.gd`) :
  `fg4_jeu_charge.jpg`, `fg4_jeu_arret.jpg` ; avant : `fg4_jeu_charge_avant.jpg`.
- Le fragment `manifest_horse.json` et l'export autonome de FG4 sont supprimés : un seul
  exportateur (`battle_fine.py figures`).
- `battle_skinned.export_mesh` lit un attribut couleur facultatif `fg_shade` (domaine sommet)
  qui multiplie la couleur de la matière (robe ombrée) ; sans effet pour les autres maillages.
- À noter pour l'orchestrateur : après la fusion de main, les scripts GDScript de SZ
  (`PrecipitationProfile`) ne sont connus qu'après un nouvel `--import` (cache de classes) ;
  rien à voir avec FG4.

## Choix de goût (bible DA, semi-réaliste)
- Pas de balzanes ni de liste peintes : tout un régiment porterait les mêmes marques (le shader
  ne tire que la robe) ; à la place, « extrémités » plus sombres (jambes, bout du nez).
- Crins de la couleur de la robe, plus sombres (bai → noirs, gris → gris foncé).
- Roncin et genet : même squelette donc même taille ; plus minces (tronc 0,90/0,83, ventre
  relevé, encolure et membres affinés 0,85/0,78).
- Selle de guerre à troussequin haut ; rênes vers la main gauche au-dessus de l'arçon.

## Limites
- Le rig `cavalry` n'a ni trot ni cabré : planches sur les clips existants (arrêt, pas, galop,
  charge, mêlée `Idle_2`, cheval abattu, chute du cavalier).
- Le caparaçon suit le tronc, pas les membres : au galop, les cuisses le traversent un peu
  (comme l'actuel). Suppression des faces cachées prudente (~5 % du corps au LOD0).
- Crinière et queue en cartes opaques doublées (pas d'alpha dans le shader) : effet « mèches »
  de près ; FG3 (matières cuites) pourra reprendre.
- Rênes rigides côté main sur l'os de selle : pour l'arc monté (rênes à droite) la main est à
  12 cm du bout des rênes.
- Même hauteur pour toutes les montures (os partagés) : un « petit » genet demanderait un rig.
- Cadavre en `c_death` : étriers rigides sur la selle, dressés quand le cheval est couché.

## Commandes
```
blender -b --factory-startup --python tools/blender_scripts/battle_fine.py -- rigs
blender -b --factory-startup --python tools/blender_scripts/battle_fine.py -- figures --only cavalry_0,cavalry_1,cavalry_2,cavalry_3,cavalry_4,cavalry_5,cavalry_6,standard_1
blender -b --factory-startup --python tools/blender_scripts/battle_fine_check.py -- <dir> --only cavalry_0,cavalry_2
blender -b --factory-startup --python tools/blender_scripts/battle_fine_cavalry.py -- --render <dir>
uv run --project tools python tools/blender_scripts/fg4_planche.py <dir>
blender -b --factory-startup --python tools/blender_scripts/battle_fine_proto.py -- horse --out <dir>   # banc FG0
```
Montés fins ≈ 10 min (8 figurines × 3 LOD, cheval reconstruit à chaque fois), rendus ≈ 6 min.

## Prochaine étape
Fusion de `feat/fg4-horse` dans `main` (avancer depuis main si FG2 a régénéré des montés : relancer
`figures` pour les huit montés), puis FG5 (A/B de performance).
