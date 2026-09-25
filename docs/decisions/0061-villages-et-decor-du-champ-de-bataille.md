# ADR 0061 — Villages et décor du champ de bataille

Date : 2026-09-25. Statut : accepté. Lot EP6 du chantier « batailles épiques » (suivi :
`docs/wip/ep6-villages-decor.md`, `docs/wip/epic.md`). S'appuie sur EP1 (taille du champ, ADR 0031),
EP3 (eau, ponts, routes, ADR 0033) et le kit de bâtiments (BR1-BR3, ADR 0021 et 0047).

## Contexte

Le champ de bataille restait un pré avec au plus un village de 6 à 11 bâtiments (B5) ou une ferme.
Le joueur veut une campagne française ou anglaise du XIVe siècle : hameaux variés, moulins, église et
cimetière, manoir à fossé, vignes, vergers, labours, meules, haies bocagères, et derrière chaque
armée son camp et ses bagages — dont le pillage coûte le moral, comme à Azincourt. Le décor doit
compter dans les règles, suivre la région et la saison, ne jamais supposer 1200 × 800, respecter
l'eau, les ponts, les gués et les routes d'EP3, rester fluide à 15 000 soldats, et se poser à la main
pour les cartes historiques d'EP7.

## Options

- **Décor de rendu seulement** (GDScript) : rien à tester, et un verger ou un manoir ne changerait
  rien au combat ; EP7 devrait tout redécrire côté Godot.
- **Étendre le village de B5** (`site.rs`) : un seul type d'objet (maisons dans un disque), pas de
  parcelles orientées, et les graines de B5 bougeraient.
- **Module de décor dans le cœur**, flux dérivé, zones orientées aux effets lus dans les données, API
  de pose explicite partagée par le générateur et EP7.

## Décision

- **Cœur** : `core/crates/sim-battle/src/decor.rs` (règles, types, requêtes, pose à la main) et
  `decor_gen.rs` (disposition). Tiré d'un flux dérivé (`DECOR_STREAM`) après l'eau et les routes :
  les tirages antérieurs sont inchangés. Le décor s'ajoute au village de B5 sans le remplacer.
  - Hameaux **en rue** le long d'une route d'EP3 (maisons des deux côtés, façade sur la rue, église
    au bout de la rue) ou **groupés** autour de l'église et de son cimetière clos (la route passe par
    le pré communal) ; **fermes isolées** en U avec un chemin jusqu'à la route ; **moulin à vent** sur
    la hauteur la plus haute trouvée, ou sur une butte levée dans la grille des hauteurs ; **moulin à
    eau** sur la berge (`Battlefield::waterside_spot` d'EP3), roue vers l'eau ; **manoir** (maison
    forte) avec cour, grange, puits et **fossé en eau** ; **vignes** (sur les pentes), **vergers**
    (près des maisons), **labours** en lanières d'états différents, **prés** (bas, près de l'eau) avec
    meules ; charrettes au bord des routes ; **haies** autour des prés et vergers dans le bocage.
  - **Camps** : derrière chaque ligne (distance au bord arrière du champ dans les données, largeur qui
    suit la largeur du champ), tentes, pavillons, feux, laager de chariots, chevaux au piquet ; convoi
    de bagages garé le long de la route la plus proche, en arrière.
  - **Contraintes de pose** : rien dans l'eau (échantillonnage de l'emprise), sur un pont ou un gué
    (marge), sur une route (distance emprise-segment), dans un bois, sur le village de B5 ni sur le
    centre des lignes de déploiement (sauf labours et prés, qui font le champ de bataille, comme à
    Azincourt). Tout est dimensionné sur `FieldSize` (nombres pour le champ standard, multipliés par
    la surface).
- **Composition** : `data/rules/battle_decor.json` (schéma `battle_decor_rules.schema.json`). La
  province choisit un paysage (vignoble : Guyenne, Bourgogne, Champagne… ; bocage : Normandie
  occidentale, Bretagne, Maine, Anjou ; openfield du Nord ; campagne anglaise aux manoirs à douves ;
  Midi aux bastides en rue et maisons de pierre ; montagne, lande, marais), sinon son terrain. La
  saison règle meules, charrettes, états des labours (labouré, semé, blé mûr, chaume), vignes
  feuillues ou nues, vergers en fleurs.
- **Règles** : un régiment dont le centre est dans une zone (hameau, cimetière, manoir, ferme,
  verger, vigne, labours, pré, camp) reçoit son **couvert** contre les traits, sa **vitesse** (labours
  plus lents sur sol détrempé), sa **défense** en mêlée (diviseur des pertes : manoir 1,5, cimetière
  1,3, hameau 1,15) et peut **briser les charges** (hameau, cimetière, manoir, ferme, vigne). Les
  bâtiments et le mobilier solide repoussent les figurines (mécanisme de BR3). Un champ nu
  (`BattleSetup::village = Some(false)` : laboratoires, sièges exclus) ne garde que les camps.
- **Pillage du camp** (`sim/camp.rs`) : un régiment ennemi en état de combattre dans un camp **sans
  garde** (aucun régiment ami valide à `guard_radius_m`) le pille en `loot_seconds` (plus vite à
  plusieurs) ; alarme (petite perte de moral) à l'entrée, puis perte de `looted_morale` pour toute
  l'armée, fatigue des pillards, journal, `SideResult::baggage_lost`. Un camp gardé ou vidé retombe.
- **Pose à la main (EP7)** : `Battlefield::place_building`, `place_windmill`, `place_watermill`,
  `place_church`, `place_manor`, `place_hamlet`, `place_plot`, `place_prop`, `place_camp`,
  `clear_decor`, et `DecorPlan` (JSON, schéma `battle_decor_plan.schema.json`, exemple
  `data/battle_maps/decor_plan_example.json`) porté par `BattleSetup::decor_plan` et appliqué après le
  décor procédural (`clear` le retire d'abord).
- **Rendu** (`game/scripts/battle/battle_decor.gd`) : modèles du kit Blender (27 nouveaux : moulin à
  eau, tentes, pavillons, meules, chariots, feux, mur de cimetière, porche, tombes, rangs de vigne,
  chevaux) en `MultiMesh` par modèle et par lot, portées de visibilité par taille ; rangs de vigne à
  deux niveaux de détail ; vergers semés avec les arbres ; parcelles peintes au sol par une texture
  `decor_fields` lue par les shaders du sol et de l'herbe ; flammes des feux en un `MultiMesh` par
  camp ; camp pillé : tentes abattues, fumée. `--no-ep6-decor` coupe le rendu (banc A/B). Pieux des
  archers : maillage de pieu écorcé et taillé, plantés irréguliers.

## Conséquences

- Les batailles de campagne ont désormais un décor qui compte : couverts et défenses à prendre,
  vignes et haies qui brisent les charges, camps à garder. L'IA prend les zones couvrantes du décor
  comme couverts (comme le village de B5) mais ne vise pas encore les camps ennemis (point ouvert).
- Les feux des camps alimentent les fumées d'EP8 (`BattleScene.add_smoke_source`) ; EP8 ne pose pas
  ses feux par défaut (`auto_campfires = false`) quand le décor a des camps. Le camp pillé fume en
  colonnes noires.
- Coût de rendu mesuré : ~+3 % de primitives, ~+90 appels, 0 à 3 % d'images par seconde à 15 000
  soldats (banc A/B `--no-ep6-decor`, `docs/wip/ep6-villages-decor.md`).
- Les batailles déjà jouées changent un peu (effets des zones, camps) ; les digests de B6 sont
  recalculés (même vainqueur).
- EP7 pose ses décors historiques par `DecorPlan` sans toucher au rendu.
