# 0153 — Bâtiments hors les murs et croissance des villes : maquettes assemblées du kit

Date : 2026-10-02. Statut : acceptée (lot TB3 du chantier TB). Plan :
`docs/design/2026-10-02-campagne-tob.md` § 3. Suit l'ADR 0152 (aucun service payant) et l'ADR 0138
(villes 1:1). Numéro réservé par l'orchestration TB ; `0153-lanceur-windows-exe.md` porte le même
numéro sur main (à renuméroter à la fusion si besoin).

## Contexte
Le plan TB3 prévoyait 24 maquettes générées (image → 3D, 8 à 12 $). L'ADR 0152 l'interdit. Le dépôt
a trois sources possibles : le kit de bâtiments Blender (`building_kit.py`, recettes `low` des
villes 1:1), les variantes GA3 (`ga3_kit.gd`) et les villages TF (variantes régionales du même kit).
Il fallait aussi décider de l'échelle, du rendu et de ce que « le bâtiment grandit » veut dire, le
moteur n'ayant pas de niveau par bâtiment.

## Décision
- **Maquettes** : `tools/blender_scripts/tb3_outbuildings.py` assemble 8 familles × 3 niveaux
  (ferme, moulin, vignoble, mine, saline, abbaye, marché, port) et un chantier à partir des
  recettes `low` du kit (chaumière, longère, grange, maison de pierre, église, manoir, halle,
  moulins, meule, puits) et de pièces procédurales écrites avec les mêmes primitives (rangs de
  vigne, œillets, jetée, grue, manège et chevalement, cloître, étals, tentes, barques,
  échafaudage). Niveau 1 : un bâtiment ; niveau 3 : un petit domaine. 154 à 2 438 triangles
  (plafonds 800 / 1 800 / 3 500). Une seule surface `Building` par maquette : même matériau atlas
  et même usure (ADR 0136) que les villes, donc « une seule main ».
- **GA3 écarté** : albédo cuit par modèle, pas de pose par le shader de relief
  (`campaign_display_height`), un matériau par modèle. **Modèles CC0 tiers écartés** : le kit
  couvre le besoin, aucune ligne de `CREDITS.md` à ajouter.
- **Niveau** : donné par `data/map/building_models.json` (schéma `building_models.schema.json`) :
  par famille, le plus haut niveau dont tous les groupes `require` comptent assez de bâtiments
  construits dans la colonie, et dont la province produit une des `resources`. Une amélioration
  remplace le bâtiment qu'elle améliore (`buildings.rs`) : les chaînes (marché → maison des
  métiers → foire) donnent directement les niveaux ; les familles sans bâtiment propre (ferme,
  saline, mine) comptent les bâtiments qui les font vivre.
- **Échelle réelle** (ADR 0138), visibles sous `render.view_range_units` = 45 (portée des villes
  détaillées, ADR 0144). `render.exaggeration.max` (1 par défaut) permet de les grossir de loin
  sans toucher au code, si le joueur veut les voir au palier moyen.
- **Rendu** : `OutbuildingLayer`, un `MultiMesh` par maillage pour tout le voisinage de la caméra
  (au plus un appel de dessin par maquette distincte), hauteurs de base en mètres posées par
  `town_building.gdshader`. Site calculé une fois par famille et par colonie (secteur propre à la
  famille, hors du bâti, hors des routes des portes, pente bornée ; grève pour le port et la
  saline) : la maquette grandit sur place.
- **Croissance de la ville 1:1** (`TownGrowth`) : le plan de 1340 reste figé ; s'y ajoutent des
  quartiers de faubourg (un par tranche de 8 % de population de la province au-dessus de 1337) et
  une enceinte (palissade, puis pierre, tours, portes) quand un bâtiment de fortification construit
  en cours de partie dépasse l'enceinte du plan et les bâtiments de départ. `replace_models`
  disparaît ; `model_holder()` reste nul (pas de nœud par colonie).
- **Suie par ville** : paramètre d'instance `town_soot` sur les nœuds de la ville 1:1, masque R8
  par ville pour le maillage lointain, `INSTANCE_CUSTOM.b` pour les instances partagées. Quantité
  (`TownSoot`) : dévastation de la province, siège, prise de la place (contrôleur changé ; saccage
  si la dévastation monte en même temps).
- **Chantier** : la maquette `worksite_1` remplace le « ⚒ » des signes (taille constante à
  l'écran, règles TB2 inchangées) et se pose à l'échelle réelle au pied de la cité en chantier.

## Conséquences
- 0 $ ; 25 GLB (2 Mo) régénérables par le script.
- À l'échelle réelle, un domaine de niveau 3 fait 3 px à d = 45 : rien n'est visible à 90 ni à
  400. L'écart avec « visible au palier moyen » du plan est assumé (ADR 0138) ; le réglage
  `exaggeration` est le levier si le joueur en décide autrement.
- Le moteur ne garde pas d'état « ville saccagée » : la suie d'une prise vit dans la session et
  ne survit pas à un rechargement (celle de la dévastation, si). Un état durable demanderait une
  règle dans `core/`, hors de ce lot.
- Les règles de niveau sont des choix de présentation : à ajuster dans les données après la
  partie pilote.
