# DN-REVUE : verdicts sur les assets générés

## Méthode
Planches-contacts étiquetées (Pillow, hors dépôt) : par asset, image retenue (`cut/s<seed>.png`), deux rendus de `sheet.png`, vue dos si présente, sujet attendu. 712 ids vus (694 du manifeste + 21 figures, moins 3 exclus déjà en reprise : siege_cannon_early_1340, siege_bombard_hooped_1380, army_lord_mounted ; 4 figures ne sont pas au manifeste). Seuil : erreurs flagrantes seulement (objet différent, composition absurde, 3D cassée, anachronisme, A-pose violée) ; ombre, teinte, texture, angle exclus. Les textures `card_*` (14) sont des cartes d'herbe/plantes sans sujet ambigu : non signalées.
Limites : les ids sans 3D (fig_army_lord_mounted_islamic, _rus, cavalry_1/4/5, standard) n'ont que l'image ; les images des deux seigneurs montés sont correctes (cavalier A-pose sur cheval).

## Flagrants (15)
| id | catégorie | problème | correctif de prompt (EN) | image fautive |
|---|---|---|---|---|
| `archer_3_jack` | figure | tient un arc (règle A-pose : rien en main) | Archer in a padded jack and sallet standing in a symmetrical A-pose, both arms raised high and open near T-pose, empty open palms, nothing held, no bow, no quiver in hand | image |
| `bombard` | siege | caisse en bois avec boulets, aucun tube de fer forgé visible (pas une bombarde) | A single large forged iron bombard: a long hooped wrought-iron barrel with a visible round muzzle opening, resting on a heavy oak timber bed, a few stone balls beside it; the iron barrel must be clearly visible, not a wooden box | image |
| `bird_starling_flying` | nature | étourneau posé sur une chaumière, fusionné avec un bâtiment (3D = cabane + oiseau) | A single common starling in flight, wings spread mid-flap, side view, alone on plain grey background, no house, no building, no perch, no ground | image |
| `cart_litter_noble` | mobile | deux mules attelées alors que le sujet est un chariot vide ; animaux fusionnés dans la 3D | A noble curtained wooden litter cart with painted roof and poles, EMPTY shafts, no horses, no mules, no animals, no people, isolated single object | image |
| `debris_ram_wreck` | debris | cadre de bélier fusionné avec une petite maison blanche | A wrecked battering ram: a split timber roof frame and a fallen iron-capped log lying beside it, isolated, no hut, no building, no stone walls | image |
| `econ_saltpan_mediterranean` | economy | image fantôme délavée, marais salants invisibles, moulin flottant ; 3D = deux huttes noires | Mediterranean solar salt pans: several flat rectangular shallow basins with white salt cones separated by low earth banks, a small windmill pump and a small stone store hut on the bank, strong clear contrast, objects fully visible and solid, on a small base | image |
| `econ_terrace_olive_grove` | economy | la 3D a perdu les couronnes des oliviers (souches noires nues) | Terraced olive grove with two ancient twisted olive trees with full grey-green leafy crowns on a dry-stone terrace wall with rock steps, single compact object | 3d |
| `env_glacier_ice_tongue` | map_extra | cabane de pierre avec un bloc de glace au lieu d'une langue glaciaire | A glacier tongue snout: blue-white crevassed ice ending in a heap of grey moraine rubble and boulders, natural landform only, no hut, no building, no roof | image |
| `env_iceberg_floe` | map_extra | maison de pierre à toit de glace au lieu d'un floe flottant | A drifting ice floe: a single jagged slab of blue-white ice with a rough top, natural ice only, no house, no building, no roof, no wall | image |
| `env_reef_rocks_awash` | map_extra | deux chaumières sur un îlot au lieu d'un récif nu | A low dark rocky reef awash: black wet rocks covered with kelp and barnacle crust, tidal flat outcrop, natural rocks only, no house, no building, no roof | image |
| `fig_sled_driver_north` | figure | traîneau fusionné aux jambes du personnage | A northern sled driver in a fur hat and tunic standing alone in A-pose, arms raised, open empty palms, feet apart on the ground; no sled, no runners, no object touching the legs | image |
| `siege_handgonne_pair` | siege | deux tonneaux fermés au lieu de deux canons à main (pas de bouche) | Two medieval hand gonnes lying side by side: each a short forged iron barrel with a visible open bore mounted on a long wooden tiller, plain, no barrels or casks | image |
| `tree_argan` | tree | 3D : tronc nu et calotte plate sombre, couronne perdue | An argan tree with a gnarled trunk and a broad round leafy dark-green crown with dense foliage, full canopy clearly attached to the trunk, single tree | 3d |
| `tree_orchard_apple` | tree | 3D : arche de fragments de tronc, couronne absente | A single apple orchard tree with a short stout trunk and a rounded dense leafy crown with a few red apples, full canopy attached to the trunk | 3d |
| `tree_pomegranate` | tree | 3D : tronc nu tordu coiffé d'un disque noir, couronne perdue | A single pomegranate tree with a twisted trunk and a rounded leafy green crown with a few red pomegranates, full canopy attached to the trunk | 3d |

Tout le reste (697 ids) : OK. Détail machine : `revue-assets.json`.


## Reprise DN-TROUS (09/10)
Les 13 flagrants non figures ont été refaits (nouvelle image fal d'après `fix_prompt`, puis TRELLIS 2, sorties dans `<id>/rv2/` ou `<id>/rv3/`, anciens artefacts intacts) et ré-ingérés : `bombard`, `cart_litter_noble`, `debris_ram_wreck`, `econ_saltpan_mediterranean`, `econ_terrace_olive_grove`, `env_iceberg_floe`, `env_reef_rocks_awash`, `siege_handgonne_pair`, `tree_argan`, `tree_orchard_apple`, `tree_pomegranate`, `bird_starling_flying` (image recadrée au-dessus de la maison), `env_glacier_ice_tongue` (image recadrée ; un reste de toit de chaume subsiste à gauche de la glace).
Z-Image ignore les négations de `fix_prompt` (« no house » fait apparaître une maison) : pour oiseau, bélier, glacier, iceberg, récif le prompt a été reformulé sans nommer l'objet indésirable.
Figures `archer_3_jack` et `fig_sled_driver_north` : nouvelles images (graine 1339, A-pose sans arc ni traîneau) et glb `3d/fal__s1339.glb` dans `<id>/rv2/` (multi-vues TRELLIS 1) ; **non cuites** : la chaîne de cuisson (`ga3_figures.py`) est sur la branche `dn/fig-bake`, non fusionnée.
