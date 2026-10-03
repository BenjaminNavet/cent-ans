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
- `tools/blender_scripts/settlements_west.py` : famille `west` (GC3c, voir plus bas).
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

## GC3c (02/10) : famille `west`, Rus' éclaircie, recentrage, yourtes — terminé

Constat en jeu : les `city_a`… d'origine (kit BR1) sont sombres et lourds (24 261 triangles) à
côté des nouvelles familles ; la Rus' était sombre aussi.

- **Famille `west`** : `tools/blender_scripts/settlements_west.py` (mêmes aides que
  `settlements_east.py`, enregistrée par `east.register_family`), 10 modèles
  `<type>_west_<a|b>.glb`. Pierre claire, tours rondes à poivrières d'ardoise bleue, maisons à
  pignon de tuile rouge-orangé (une sur cinq en ardoise), chaume blond à la campagne. Les anciens
  `city_a.glb`… ne sont ni modifiés ni supprimés (batailles, autres scripts).
- **Rus'** : `Log` miel, `Shingle` gris argenté, murs `WhiteStone`, bulbes `Gilt` /
  `CopperGreen` saturés et `DomeBlue` (bourg b, abbaye a). `Log` éclaircit aussi palissades et
  enclos de la steppe et les troncs de palmiers.
- **Recentrage** : `settlements_east.centre_footprint` (appelé à l'export et par la planche)
  centre l'emprise au sol de tous les modèles des six familles sur l'origine. Faubourgs resserrés
  (`suburb(step=0.1)`), palmeraie de `city_isl_a` rapprochée : toutes les villes ≤ 3,64.
- **Yourtes** : mur bas, bandeau rouge sous l'avant-toit débordant, toit conique, couronne
  sombre, porte colorée (tente du khan : toit doré, bandeau turquoise).
- **Planche** : `settlements_sheet.py -- west` rend la nouvelle famille ; l'ancien kit BR1
  s'appelle désormais `legacy`. La ligne `SIZE` donne aussi le décalage du centre retiré.

### Triangles et emprise (largeur × profondeur) après export, centre à 0,000
| famille | city a / b | town a / b | castle a / b | abbey a / b | village a / b |
|---|---|---|---|---|---|
| west (triangles) | 2 494 / 2 196 | 1 108 / 1 278 | 668 / 942 | 604 / 652 | 502 / 526 |
| west (emprise) | 3,54×3,02 / 3,09×3,31 | 1,99×1,95 / 1,94×1,94 | 1,82×1,39 / 1,79×1,86 | 1,97×1,73 (a, b) | 2,10×2,09 / 2,07×2,05 |
| rus (triangles) | 2 900 / 2 914 | 1 276 / 1 238 | 766 / 1 034 | 1 302 / 1 282 | 712 / 692 |
| steppe (triangles) | 3 338 / 2 570 | 1 772 / 1 816 | 1 284 / 1 300 | 756 / 610 | 1 032 / 1 044 |

Villes des autres familles : med 3,63 / 3,36 ; byz 3,40 / 3,39 ; rus 3,19 / 3,36 ; isl 3,64 /
3,31 ; steppe 3,57 / 3,44. Décalages retirés les plus forts : `city_rus_b` 0,44, `city_west_a`
0,27 (le noyau muré est donc décalé d'autant par rapport au point de la carte).

### Points ouverts GC3c
- Câblage : `town_maquette_data.gd` nomme la famille par défaut `<type>_<a|b>` ; pour afficher
  `west` il faut que la famille par défaut pointe sur `<type>_west_<a|b>` (à l'orchestrateur), et
  créer les `.import` des 10 nouveaux `.glb`.
- Planches regardées : west ×3, rus ×1, steppe ×2 (budget de 6 atteint). La planche rus finale
  (bardeaux un peu assombris après la première vue, 0,45/0,47/0,50) n'a pas été revue.
- Yourtes : de trois quarts elles se lisent comme des tambours blancs à bandeau rouge et chapeau
  conique ; lecture en vue zénithale de jeu à juger dans Godot.
- `west` : 8 à 12 surfaces par modèle (une par teinte), comme les autres familles.
- Abbaye `west` 1,97 de large pour un contrat de 2,2 (les autres familles : 2,00 à 2,28).

## GC6-perf (02/10) : une surface par modèle
`settlements_east.bake_kit` : teintes de palette cuites en couleur de coin (`COLOR_0` linéaire,
alpha 0 sur les bannières), matériau unique `Kit`, rugosité 0,9 ; plus d'atlas ni d'UV pour les
six familles (`KIT_TINTS` : couleur unie des pièces `Wood`). Les « 5 à 12 surfaces par modèle »
des points ouverts ci-dessus sont réglées. La planche applique la même finition (elle montre ce
que Godot dessine). Détail et mesures : `docs/wip/gc-carte-generalisee.md`, section GC6-perf.

## Prochaine étape
Aucune dans ce lot. À l'orchestrateur : import Godot (`.import`), câblage de la famille `west`
comme défaut (GC2), jugement en jeu.
