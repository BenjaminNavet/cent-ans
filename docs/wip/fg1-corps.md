# FG1 — Corps humain des figurines fines (MakeHuman en production)

Branche : `feat/fg1-corps` (worktree agent). Plan : `docs/wip/fg-figurines-fines.md`.
Prototype : `docs/wip/fg0-prototype.md`.

## État
- [x] Rig aux proportions réalistes (`battle_fine_rig.py`), rigs `human` / `cavalry` recuits
  dans `game/assets/models/battle_fine/` (mêmes os, mêmes clips ; os du cheval intouchés)
- [x] Arc long : main d'arc placée sur la ligne ancre (commissure droite) + visée, torse
  tourné à 60° ; la corde et la main droite finissent à la joue (±3 cm)
- [x] Visages : 8 clés de forme MPFB dans `fg_base_male.blend` (`fg_makehuman_base.py`,
  `FACES`), une tête par variante au LOD0 ; cheveux courts et barbes en coques
- [x] Recettes fines (`battle_fine_figures.py`) : tenues par pièces Quaternius (harnois,
  tunique, paysan, capuche), équipement V2 reconstruit sur le corps, kit FG0 pour
  `infantry_0` / `cavalry_0`, tabard drapé
- [x] Drapeau `--fine-figures` dans `BattleSkinned` (manifeste fusionné, rigs `fine_*`)
- [x] 28 recettes exportées (toutes : fantassins, tireurs, cavaliers, porte-étendards,
  musiciens, servants), vérifiées sur les données cuites (`battle_fine_check.py`,
  `docs/img/fg/fg1_poses_cuites.png`)
- [x] Captures en jeu `docs/img/fg/fg1_melee_6m.png`, `fg1_melee_12m.png`,
  `fg1_closeup_26m.png` (défaut | `--fine-figures`) ; smoke test OK ; imposteurs cuits en
  jeu depuis le LOD0 fin (rien à changer, vérifié : 85 régiments imposteurs, 8 atlas)

## Proportions retenues (rig, pose de liaison)
| Segment | Quaternius | FG1 |
|---|---|---|
| hanches → cou | 0,60 m | 0,63 m (×1,05 sur Abdomen/Torso/Chest/Neck) |
| cou → tête (pivot) | 0,074 m | 0,097 m |
| épaule (articulation) | x ±0,15, z 1,37 | x ±0,18, 3 cm plus bas |
| humérus | 0,176 m | 0,251 m |
| avant-bras | 0,228 m | 0,256 m |
| jambes, pieds | inchangés | inchangés (pieds enfants de `Root`, translations animées) |

Corps MakeHuman ajusté à l'échelle 0,955 (tronc) au lieu de 0,904 : 1,70 m, tête à sa
taille réelle (la grosse tête Quaternius n'est pas reproduite : corrigée par l'échelle du
corps, pas par le rig).

## Triangles (manifeste `battle_fine/manifest.json`)
| Famille | LOD0 | LOD1 | LOD2 |
|---|---|---|---|
| à pied (22 recettes) | 8 100-12 300 | 1 380-2 240 | 280-410 |
| montées (cavalier + cheval Quaternius + harnachement) | 8 000-11 300 | 1 620-2 240 | 390-550 |
Toutes variantes comprises (têtes, casques et armes masqués par variante comptés). Cibles :
tête 1 300 / 320 / 56 (une par variante au LOD0, ≤ 2 800 au total), coques de vêtements
2 200 / 440 / 70 ; LOD2 : yeux, cheveux, barbes, ceinture supprimés, pieds pris au corps.

## Primitives `--units=50` (`--benchmark --bench-at=90`, 1600×900, ~11 900 soldats)
| | défaut | `--fine-figures` |
|---|---|---|
| primitives | 3,13 / 3,19 M | 3,51 / 3,52 M (+11 %) |
| i/s moyens (machine partagée, bruit ±30 %) | 27,0 / 30,2 | 18,0 / 24,9 |
Le coût est surtout sommet (LOD0 ×4, LOD1 ×3,5 en sommets) : à reprendre en FG5 (relais
LOD0 à 16-20 m, LOD1 plus léger, ombres sur LOD2).

## Choix de goût (tranchés, esprit bible DA)
- Taille 1,70 m, tête réelle ; épaules 3 cm plus basses et plus larges que Quaternius.
- Visages : jeune homme (base FG0), vétéran 45 ans nez cassé, jeune 18 ans, mâchoire carrée,
  visage long émacié, cogneur nez écrasé, aquilin menton saillant, vieux soldat 55 ans ;
  barbes courtes pour 3 visages sur 8 ; cheveux coupés courts au-dessus des oreilles.
- Tenues : harnois (haubert, chausses, surcot fendu du FG0 ou cuirasse blanche), tunique
  (gamboison + pièce de livrée, jupe à mi-cuisse), paysan (cotte roussâtre à manches de lin,
  jupe au genou, chapeau de paille), capuche (cotte sombre, manches de mailles, chaperon).
- Tabard : drapé sur la tunique (coques) au lieu des deux panneaux plats Quaternius.

## Écarts, limites
- Os du cheval non touchés (FG4) : les montés gardent le cheval Quaternius ; `cavalry.bones.bin`
  fin recuit avec les étriers actuels. FG4 : brancher le nouveau cheval et les étriers élargis
  dans `battle_fine.py` (`FineMount` sous-classe `battle_skinned_cavalry.Mount`), puis
  relancer `rigs` + `figures`.
- Arc long : la main d'arc n'atteint pas l'allonge réelle (tirage effectif 0,55 m au lieu de
  0,68) ; l'ancre à la joue est tenue (±3 cm), la flèche est donc un peu courte.
- Piques : poings à ~5-10 cm du fût selon l'image (comme la version Quaternius).
- Équipement V2 (casques, jacques, brigandines, écus) reconstruit tel quel sur le nouveau corps :
  il tombe juste mais garde son facettage (FG2). Le kit FG0 n'équipe que `infantry_0` et
  `cavalry_0`.
- Matières : couleur + code par sommet seulement (cartes cuites en FG3). Le teint vient du
  shader (variation par soldat) ; mains gantées de cuir pour le harnois.

## Commandes
```
blender -b --python tools/blender_scripts/fg_makehuman_base.py          # MPFB requis (visages)
blender -b --factory-startup --python tools/blender_scripts/battle_fine.py -- rigs
blender -b --factory-startup --python tools/blender_scripts/battle_fine.py -- figures [--only infantry_0]
blender -b --factory-startup --python tools/blender_scripts/battle_fine_check.py -- <dir>
godot --path game -- --fine-figures
```

## Prochaine étape
Lot terminé (branche non fusionnée). Suites : FG2 (équipement fin par recette), FG3 (cartes
cuites), FG4 (cheval), FG5 (perf, passage par défaut).
