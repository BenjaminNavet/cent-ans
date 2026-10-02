# 0162 — Bâtiments hors les murs et croissance des villes : maquettes assemblées du kit, taille tenue à l'écran

Date : 2026-10-02. Statut : acceptée (lot TB3 du chantier TB). Plan :
`docs/design/2026-10-02-campagne-tob.md` § 3. Suit l'ADR 0152 (aucun service payant) et l'ADR 0138
(villes 1:1). Écrit sous le numéro 0153, déjà pris sur main par `0153-lanceur-windows-exe.md` :
renuméroté 0162. Amendé le 2026-10-02 après lecture des captures de contrôle (voir « Taille tenue
à l'écran »).

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
  échafaudage). Niveau 1 : la pièce signature ; niveau 3 : un ensemble riche. 104 à 1 555
  triangles (plafonds 800 / 1 800 / 3 500 ; signes de colonie ≤ 6 000). Une seule surface `Building` par maquette : même matériau atlas
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
- **Taille tenue à l'écran** (amendement, `render.screen`) : l'échelle réelle de l'ADR 0138 ne
  vaut plus que de près (distance du rig ≤ `real_below` = 10). À partir de `full_from` = 15, chaque
  maquette hors les murs garde une largeur d'écran selon son niveau (`fractions` de la hauteur de
  l'écran : 4,4 %, 6 %, 8 %, soit 40, 54 et 72 px au centre d'un écran de 900 px ; ≥ 28 px
  autour du point visé une fois la perspective comptée ; niveau 3 = 1,82 × niveau 1) : son
  grossissement est proportionnel à la distance du rig, comme les figurines FK et les fosses de
  TB4. Fondu de sortie entre `fade_from_units` = 120 et `view_range_units` = 160. Les faubourgs
  ajoutés grossissent de même (quartier entier, depuis son départ sur la route) ; l'enceinte
  ajoutée garde son tracé sur la ville et épaissit (épaisseur tenue à l'écran, au plus 20 % du
  rayon de la ville, tours éclaircies).
  **Pourquoi l'exagération l'emporte ici sur l'ADR 0138.** L'ADR 0138 pose les villes à l'échelle
  1:1 parce qu'une ville est un objet de plusieurs centaines de mètres, lisible de près, et
  relayée de loin par son signe et son nom. Un bâtiment hors les murs fait 20 à 120 m : à
  l'échelle réelle il tient en moins de 8 px à la distance 15 et disparaît aux distances où l'on
  joue (15 à 120), alors que le but du lot est justement que le joueur **voie** ce qu'il a
  construit (lecture à la Thrones of Britannia). Ces maquettes sont donc des signes de carte en
  volume, pas du décor à l'échelle : elles suivent la règle des signes (taille d'écran tenue),
  pas celle du décor. Les villes 1:1 elles-mêmes ne changent pas.
- **Mise en place de loin** : les maquettes grossies s'écartent de la ville et les unes des
  autres dans la même proportion. Chaque emprise est un disque dont l'écart au bord de la ville
  et le rayon suivent la distance du rig ; la mise en place est calculée par palier géométrique
  de distance (`band_ratio` 1,5), à la borne haute du palier : anneau au bord de la ville, dans
  la direction du site réel de la maquette (rive pour le moulin, grève pour le port), puis
  angles voisins et second anneau. Un site est refusé en mer, dans un lit de fleuve, sur
  l'emprise d'une autre colonie ou d'une emprise déjà posée (essai refait jusqu'au bas du
  palier) ; route principale, route d'une porte ou fleuve sous l'emprise : pris seulement faute
  de mieux. Les cités sont servies avant les villes, les niveaux hauts avant les bas ; une
  maquette qui ne tient pas est retirée à ce palier, et au-delà de `minor_until` = 60 seules les
  cités et les villes gardent les leurs. Mise en place gardée par colonie tant que ses maquettes
  ne changent pas.
- **Maquettes redessinées en signes en volume** (troisième passe, après lecture des captures) :
  les premiers domaines « réalistes » (petits bâtiments sur une grande emprise) ne se lisaient
  pas à 40-70 px. Chaque maquette est maintenant faite de quelques pièces grosses et hautes avec
  une silhouette propre (ailes du moulin, roue, clocher, halle et étals de couleur, grue et nef
  à voile, chevalement et terril, bassins blancs, rangs de vigne), bâtiments du kit agrandis
  (× 1,3 à 2), pièces hautes au fond, basses devant ; volumes relevés de 30 % de loin
  (`screen.vertical`), teintes éclaircies (`brighten`), usure réduite (`aging`), façade tournée
  vers la caméra de jeu (vue du sud), ou vers l'eau pour un port (± 70° au plus).
- **Signe de colonie** (`screen.signs`, maquettes `sign_village`, `sign_town`, `sign_walled`,
  `sign_city`, `sign_castle`) : de loin la ville 1:1 n'est qu'une empreinte ; une maquette tenue
  à l'écran la recouvre (toits serrés, église, enceinte et tours si la ville est murée), plus
  grosse que toute maquette hors les murs (cité 11,5 % de la hauteur d'écran). Elle n'apparaît
  que si elle dépasse la ville réelle (jamais sur une cité emblématique vue de près) et s'efface
  sous 15. C'est un **second écart à l'ADR 0138**, de même nature que le premier : la ville 1:1
  reste la ville de près, le signe la relaie aux distances de jeu. `signs: {}` le retire.
- **Pose par pièce sur le relief** : une maquette tenue à l'écran couvre plusieurs kilomètres ;
  posée à l'altitude de son centre, elle était enterrée dans les versants. Le manifeste liste
  les pièces rigides de chaque maquette (`pieces`, écrites par le script Blender) ; Godot écrit
  le centre de sa pièce dans `CUSTOM0` de chaque sommet et `town_building.gdshader` (`drape`) lit
  la heightmap du terrain sous ce centre. `drape` suit la part de tenue à l'écran (0 de près).
- **Port à la côte** : de loin, le port se pose sur la grève la plus proche (`shore_reach_far`
  = 12 unités), même si la ville n'est pas au bord de l'eau ; sinon dans l'anneau.
- **Brouillard de guerre** : rien n'est posé (maquettes, faubourgs, enceinte ajoutée) dans une
  province hors de vue (`ArmyMarkers.hidden_provinces`, rempli par `MinimapController`) ; la
  ville garde son signe (elle reste connue du joueur).
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
  l'écran, règles TB2 inchangées) et se pose au pied de la cité en chantier, comme une maquette
  de niveau 1.

## Conséquences
- 0 $ ; 25 GLB (2 Mo) régénérables par le script.
- De loin, une maquette de niveau 3 couvre plusieurs kilomètres de carte (7 unités à la distance
  90) : la carte n'est plus à l'échelle autour des villes, c'est assumé. Dans les régions denses,
  toutes les maquettes ne tiennent pas : les moins prioritaires manquent à ce palier et
  reviennent quand on s'approche.
- La largeur à l'écran est tenue d'après la distance du rig, pas d'après la profondeur de chaque
  maquette : le fond de l'image est plus petit (perspective), le premier plan plus grand.
- Les positions changent avec la distance ; d'un palier à l'autre une maquette peut changer
  d'angle autour de sa ville quand la place manque.
- Le moteur ne garde pas d'état « ville saccagée » : la suie d'une prise vit dans la session et
  ne survit pas à un rechargement (celle de la dévastation, si). Un état durable demanderait une
  règle dans `core/`, hors de ce lot.
- Les règles de niveau sont des choix de présentation : à ajuster dans les données après la
  partie pilote.
