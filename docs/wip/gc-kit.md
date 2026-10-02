# GC3 — kits de l'Est et du Sud (maquettes de colonies)

Lot GC3 du chantier GC (ADR 0158, plan `docs/wip/gc-carte-generalisee.md`). Worktree
`../gp-gc-kit`, branche `feat/gc-kit`.

## But
Cinq familles d'architecture vers 1340 (`med`, `byz`, `rus`, `isl`, `steppe`), 5 types × 2
variantes : `game/assets/models/settlements/<type>_<famille>_<a|b>.glb`.

## Fichiers
- `tools/blender_scripts/settlements_east.py` : palette, aides (tour, courtine, coupole, yourte…),
  une fonction par famille et par type (`build_<famille>_<type>(variante)`), table `MODELS`.
- `tools/blender_scripts/settlements.py` : importe `settlements_east.MODELS`, budget par modèle.
- `tools/blender_scripts/settlements_sheet.py` : planche de contrôle d'une famille
  (`blender --background --python settlements_sheet.py -- <famille>`) →
  `docs/img/gc/kit_<famille>.png` (non suivi), lignes `SIZE <nom> <l> <p> <h> <triangles>`.

## Choix
- Maisons en primitives simples (pas les maisons du kit BR1) : peu d'éléments, gros, lisibles de
  loin, peu de triangles.
- Nouvelles teintes = matériaux unis de la palette, hors couches d'atlas : Godot garde leur couleur
  plate (`BuildingMaterials.remap_mesh` ne touche pas les noms inconnus). Les matières héritées
  (`Stone`, `Wood`…) passent toujours par l'atlas `Building`.

## État (02/10) : terminé
- [x] Squelette : module, table des 50 modèles, planche.
- [x] `med`, `byz`, `rus`, `isl`, `steppe` : 10 modèles chacun, exportés dans
  `game/assets/models/settlements/` (sans `.import` : à créer par l'orchestrateur).
- [x] Contrat vérifié sur les `.glb` (un maillage, un nœud, `Banner`, fondations sous 0, budgets).
- [x] Planches `docs/img/gc/kit_<famille>.png` regardées (5 + 2 après corrections groupées).

### Triangles (a / b)
| famille | city | town | castle | abbey | village |
|---|---|---|---|---|---|
| med | 1 752 / 1 552 | 760 / 786 | 908 / 650 | 612 / 768 | 740 / 818 |
| byz | 2 568 / 2 156 | 1 662 / 1 388 | 1 342 / 1 208 | 974 / 884 | 602 / 594 |
| rus | 2 900 / 2 914 | 1 276 / 1 238 | 766 / 1 034 | 1 302 / 1 282 | 712 / 692 |
| isl | 2 766 / 2 906 | 1 398 / 1 466 | 1 064 / 1 086 | 1 052 / 1 152 | 864 / 992 |
| steppe | 2 938 / 2 330 | 1 500 / 1 576 | 1 140 / 1 172 | 692 / 562 | 888 / 932 |

Emprises (plus grande dimension au sol) : villes 3,27 à 3,86 (Ouest 3,18 à 3,61), bourgs 1,88 à
2,06, châteaux 1,82 à 1,94, abbayes 2,00 à 2,28, villages 1,76 à 2,11.

### Corrections après planches
- `byz` : coupoles de plomb partout (la tuile sur brique ne se lisait pas), contreforts de la grande
  église abaissés ; fût de tour en double laissé hors du maillage joint (détecté par la
  vérification des `.glb`, garde ajoutée dans `settlements.export_model`).
- `rus` : bardeaux assombris (se confondaient avec la pierre blanche).
- `isl` : dessus coplanaires des galeries de cour et des cellules du ribat (carrés noirs).
- `steppe` : yourtes plus basses avec couronne sombre, enclos au sol clair, talus enherbés.

## Points ouverts
- Surfaces par modèle : 5 à 12 (Ouest : 3 à 6), une par teinte unie ; à regrouper (atlas de
  couleurs ou couleurs de sommet) si GC2 mesure trop d'appels de dessin par MultiMesh.
- Les teintes ne sont jugées que dans Blender (Cycles, vue « Standard ») ; rendu Godot non vérifié.
- `city_isl_a` (3,86) et `city_med_a` (3,75) dépassent un peu 3,6 par leur faubourg.
- Yourtes : lues comme des pastilles blanches à couronne sombre ; forme de tente peu marquée de
  très haut.

## Prochaine étape
Aucune dans ce lot. À l'orchestrateur : import Godot (`.import`), câblage des familles dans
`data/art/town_maquettes.json` (GC2), jugement en jeu.
