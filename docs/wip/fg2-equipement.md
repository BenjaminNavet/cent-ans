# FG2 — Équipement fin des figurines de bataille

Branche : `feat/fg2-equipment` (worktree agent). Plan : `docs/wip/fg-figurines-fines.md`.
Précédents : `docs/wip/fg1-corps.md`, `docs/wip/fg0-prototype.md`. main (FG4, 0a53488e)
fusionné dans la branche le 26/09 ; `rigs`, `figures` (28) et `check` relancés après.

## État : terminé (branche non fusionnée)

## Code
- `tools/blender_scripts/battle_fine_gear.py` : registre `GEAR` (constructeur fin par nom
  d'objet V2, repli sur V2 sinon), aides (révolution de profils, balayage, rivets, ourlets
  `hem`, fentes `cut_slit`, UV `unwrap`), casques, armures (plates des membres par époque,
  jaque, brigandine), faces cachées (`cull_hidden`).
- `tools/blender_scripts/battle_fine_weapons.py` : armes et écus (même placement que V2 :
  repère `Prop`, poing gauche, avant-bras, dos).
- `battle_fine_figures.py` : branchement, harnois par époque, ourlets, faces cachées
  (`hide_covered`, `REPLACED_BY`), rôles de budget (`GEAR_ROLES`), plafond par LOD
  (`fit_budget` : 11 900 / 1 980 / 345 à pied, cavalier 9 000 / 1 500 / 250 hors cheval).
- `battle_fine_equipment.py` : jupe FG0 à douze plis arrondis (`_skirt(folds=)`).
- `battle_fine_rig.py` : arc long, torse à -75° (FG1 -60°), épaule d'arc poussée de 20°.

## Pièces par recette
| Recette | Casques | Armure | Armes / écus |
|---|---|---|---|
| infantry_0 | bassinet pointu + camail | haubert, surcot plissé, harnois début (2 lames, cubitières, canons, genouillères, grèves) | épée XVI, écu, fourreau |
| infantry_1 | chapel, bassinet ouvert, feutre | gamboison (FG1) | pique à attelles |
| infantry_2 | feutre, chapel, chapeau de paille | tabard ourlé | lance, vouge, fourche |
| infantry_3 | feutre, chapel | tunique galloise | lance, bouclier à umbo |
| infantry_4 | feutre, chapel, bassinet + camail | jaque matelassé | pique, targe cloutée (dos) |
| infantry_5 | chapel, **cervelière** | tabard | goedendag fretté |
| infantry_6 | salade + bavière / salade d'archer | brigandine rivetée (2 couleurs) | coustille |
| infantry_7 | bassinet à visière + camail, salade + bavière | harnois tardif (3 lames, garde-bras, gantelets) | hache d'armes |
| infantry_8 | chaperon, **cervelière**, chapel | brigandine, manches de mailles | épée, rondache peinte |
| archer_0 / cavalry_2 | chapel, feutre | tunique | arc long, flèche empennée, sac à flèches |
| archer_1 / archer_2 / archer_4 | chapel, bassinet ouvert/camail | gamboison, tabard | arbalète (arc d'acier, étrier), étui, pavois à arête |
| archer_3 | salade d'archer, feutre | hoqueton (jaque) de livrée | arc long |
| archer_5 | salade, feutre | jaque | couleuvrine, poire à poudre (V2) |
| cavalry_0 | heaume, bassinet + camail | harnois début | écu, lance de guerre + pennon fourché |
| cavalry_1 / 4 / 6 | chapel, bassinet, salade, feutre | gamboison, brigandine, jaque | lance |
| cavalry_3 | salade + bavière, bassinet à visière | harnois blanc (cuirasse, cuissots, gantelets) | lance |
| cavalry_5 | feutre, chapel | tunique | adarga, javeline |
| standard_0/1 | bassinet ouvert/camail, chapel | harnois début | hampe (V2) |
| musician_0/1, crew_0/1 | feutre, chapel | tabard ourlé, tunique | tambour, baguettes, busine, écouvillon (V2) |

## Triangles (manifeste)
| Famille | LOD0 | LOD1 | LOD2 |
|---|---|---|---|
| à pied (21) | 9 380-11 872 | 1 784-1 968 | 320-345 |
| montés (7 + standard_1), cheval FG4 compris | 15 193-17 414 | 2 749-2 933 | 599-650 |

## Mesures `--units=50 --benchmark --bench-at=90` (1600×900, ~11 950 soldats)
| | défaut | `--fine-figures` |
|---|---|---|
| primitives | 3,19 / 3,13 M | 3,55 / 3,56 M (+12 %, FG1 +11 %) |
| i/s moyens (bruit ±20 %) | 28,4 / 26,0 | 31,9 / 28,6 |

## Captures
`docs/img/fg/fg2_planche_equipement.png` (gros plans Blender), `fg2_gros_plan.png`,
`fg2_melee_6m.png`, `fg2_melee_12m.png`, `fg2_archers_tir.png` (rangs `v2_figures_shot`, LOD0),
`fg2_melee_26m.png` (bataille, `--closeup`) ; toutes « défaut | `--fine-figures` ».

## Choix de goût (tranchés, bible DA)
- Harnois « début » (1340-1360) sur les hommes d'armes et chevaliers de base : spalières à
  deux lames, cubitières, canons, genouillères, grèves sur mailles ; « tardif » (1415-1450)
  sur la retenue anglaise et les gendarmes ; cuissots seulement sans jupe de cotte.
- Cervelière à la place du bassinet ouvert pour les Flamands et routiers (1302-1370).
- Mailles éclaircies (0,44 au lieu de 0,34) : le motif du shader assombrit d'un tiers.
- Vouge (lame en couperet, pointe, croc) pour le « bill » des milices françaises.
- Flèche visible de 0,62 m + pointe : tirage réel du rig ~0,59 m.

## Limites
- Tirage de l'arc 0,55 -> 0,59 m seulement (bras du rig 0,51 m) ; flèche raccourcie en
  conséquence. Un tirage réel (0,72 m) demanderait des bras plus longs (FG1) ou une pose de
  tronc tordue en avant.
- Écus et pavois : orientation V2/FG0 conservée (pointe de l'écu vers le coude au repos).
- Casques sans martelage ni gravure : cuisson FG3. Fentes et ventaux par faces sombres.
- UV : une projection par pièce (hors faces peintes) ; les coques gardent les UV MakeHuman.
- Faces cachées supprimées d'après la pose de repos : un fente/pli peut laisser voir le vide en
  mouvement extrême (non constaté sur les captures).
- Barbes FG1 très sombres sur certains visages (hors lot).
- `cavalry.bones.bin` recuit (pose d'arc) : à relancer après fusion si FG4/FG5 le recuit aussi.

## Commandes
```
blender -b --factory-startup --python tools/blender_scripts/battle_fine.py -- rigs
blender -b --factory-startup --python tools/blender_scripts/battle_fine.py -- figures [--only a,b]
blender -b --factory-startup --python tools/blender_scripts/battle_fine.py -- check <dir>
blender -b --factory-startup --python tools/blender_scripts/battle_fine_check.py -- <dir>
```

## Prochaine étape
Fusion par l'orchestrateur ; FG3 (cartes cuites, UV prêtes), FG5 (perf).
