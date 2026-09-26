# FG4 — Cheval fin en production

Branche : `feat/fg4-horse` (worktree agent, non fusionnée). Plan : `docs/wip/fg-figurines-fines.md`.
Prototype : `docs/wip/fg0-prototype.md`.

## État : terminé (en attente de fusion avec FG1)
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
- [x] Chaîne LOD exportée en `CAM1` dans `game/assets/models/battle_fine/` + fragment de manifeste.
- [x] Planches `docs/img/fg/fg4_clips.png` (destrier caparaçonné), `fg4_clips_cavalry_1.png`
  (roncin, jambes visibles), `fg4_chevaux.png` (montures, robes, tête, LOD) ; captures en jeu
  `fg4_jeu_charge.jpg`, `fg4_jeu_arret.jpg` et `fg4_jeu_charge_avant.jpg` (actuel).

## Triangles (cheval seul / figurine complète avec cavalier actuel)
| Figurine | Monture | LOD0 | LOD1 | LOD2 |
|---|---|---|---|---|
| cavalry_0 chevaliers | destrier caparaçonné | 5 510 / 9 681 | 1 087 / 1 792 | 279 / 651 |
| cavalry_1 sergents | roncin | 5 716 / 8 884 | 1 128 / 1 650 | 284 / 557 |
| cavalry_3 gendarmes | destrier, chanfrein, flançois ou caparaçon | 5 716 / 10 147 | 1 128 / 1 839 | 284 / 626 |
| cavalry_5 jinetes | genet | 5 716 / 8 629 | 1 128 / 1 631 | 284 / 551 |
| standard_1 | destrier caparaçonné | 5 510 / 9 807 | 1 087 / 1 786 | 279 / 623 |

(`cavalry_2`, `4`, `6` comme `cavalry_1` à ± 200 près ; détail dans `manifest_horse.json`,
champs `tris` et `horse_tris`.) Cheval LOD0 : corps 3 700, crinière 2 × 600, queue 2 × 350 (cartes
doublées : ni alpha ni double face dans le shader), yeux 120. Harnachement LOD0 : ~1 550 sans
barde, ~2 470 avec caparaçon ; LOD1 145-325 ; LOD2 ~80-125. Actuel : cavalry_0 2 956 / 746 / ~370.

## À raccorder à la fusion (orchestrateur)
1. **Chargement** : `battle_fine/manifest_horse.json` a la même forme que
   `battle_skinned/manifest.json` → `figures` (entrées `cavalry_0`-`6`, `standard_1` : `rig`,
   `lods`, `tris`, `variants`, `style`, `noble`, `pole_top`/`pole_axis`, plus `horse` et
   `horse_tris`). Le chargeur `--fine-figures` de FG1 doit fusionner ces entrées (priorité aux
   fichiers fins) et prendre le rig `cavalry` (os, clips, `cavalry.bones.bin`) dans
   `battle_skinned/` (champ `bones` du fragment). Fichiers : `battle_fine/cavalry_*_lod*.mesh.bin`,
   `battle_fine/standard_1_lod*.mesh.bin`. Rien côté Godot n'a été modifié ici.
2. **Cavalier fin** : aujourd'hui `battle_fine_cavalry.build_figure` prend le cavalier actuel de
   `battle_skinned_cavalry.build_cavalry` (groupes `R:`). Quand FG1 fournit son cavalier monté,
   remplacer ces deux lignes par la construction FG1 sur `mount.rarm` (mêmes groupes `R:`) ;
   `fine_horse` et `harness` ne changent pas.
3. **Texture d'os** : FG1 allonge les os `R:` → après fusion, recuire une fois :
   `blender -b --factory-startup --python tools/blender_scripts/battle_skinned.py -- --only none`
   (les deux rigs ; garde `STIRRUP_HALF_WIDTH`), puis
   `blender -b --factory-startup --python tools/blender_scripts/battle_fine_cavalry.py -- --render <dir>`
   et `uv run --project tools python tools/blender_scripts/fg4_planche.py <dir>`.
4. `battle_skinned.export_mesh` lit un attribut couleur facultatif `fg_shade` (domaine sommet)
   qui multiplie la couleur de la matière : conflit possible si FG1 touche la même fonction.
5. `battle_fine_proto.build_cavalry` : 3 lignes d'étriers retirées (désormais dans `Mount`).

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
blender -b --factory-startup --python tools/blender_scripts/battle_fine_cavalry.py -- [--only cavalry_0] [--render <dir>] [--no-export]
uv run --project tools python tools/blender_scripts/fg4_planche.py <dir>
blender -b --factory-startup --python tools/blender_scripts/battle_fine_proto.py -- horse --out <dir>   # banc FG0
```
Export complet ≈ 9 min (8 figurines × 3 LOD, cheval reconstruit à chaque fois), rendus ≈ 6 min.
Capture en jeu faite en copiant temporairement les `.mesh.bin` fins sur ceux de `battle_skinned/`
(`tests/v2_figures_shot.gd`), fichiers restaurés ensuite.

## Prochaine étape
Fusion avec FG1 (points 1-3 ci-dessus), puis FG5 (A/B de performance).
