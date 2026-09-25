# ADR 0033 — Eau, ponts et routes du champ de bataille

Date : 2026-09-25. Statut : accepté. Lot EP3 du chantier « batailles épiques » (suivi :
`docs/wip/ep3-eau-chemins.md`, `docs/wip/epic.md`).

## Contexte

La rivière du champ de bataille était une bande de largeur fixe avec deux gués, sans pont ni route ;
l'eau ne changeait presque rien aux combats et l'IA l'ignorait. Les batailles de la guerre de Cent Ans
se jouent pourtant souvent sur un passage (pont, gué de Blanchetaque, ruisseau de Crécy, vallée de
Poitiers). Il faut plusieurs cours d'eau, des ponts qui font goulot, des gués nombreux ou rares, des
routes qui comptent, et une IA qui les tient ou les force — sans casser les graines existantes ni
supposer un champ de 1200 × 800 m (EP1 rend la taille du champ paramétrique).

## Options

- **Eau de rendu seulement** (rivières dessinées dans Godot) : rien à tester, et le joueur verrait des
  passages qui ne comptent pas.
- **Grille de franchissabilité générique** (coût par case, A* complet) : général, mais coûteux pour
  des milliers de figurines, et les règles propres au pont (goulot, tête de pont) resteraient à écrire.
- **Réseau hydrographique explicite dans le cœur** : rivière de largeur variable, affluent, ruisseaux,
  bras mort, ponts et routes comme objets ; requêtes ponctuelles (`water_kind`, `bridge_at`,
  `road_at`, `crossings`) et itinéraire par le passage le moins coûteux.

## Décision

- **Cœur** (`core/crates/sim-battle/src/hydro.rs`) : réseau explicite tiré de flux dérivés
  (`HYDRO_STREAM`, `ROADS_STREAM`) ; les tirages des batailles antérieures sont inchangés. Largeur par
  terrain (10-40 m, variation le long du cours), gués et ponts selon la largeur (rivière étroite :
  3-4 gués ; large : 0-1 gué et 1-3 ponts), ponts de bois (4-6 m) ou de pierre (5-8 m, rivières
  larges), berges escarpées ou marécageuses, bras mort, affluent avec passerelle, ruisseaux. Routes par
  chaque pont et la plupart des gués, d'un bord à l'autre du champ.
- **Règles** (`sim/water.rs`) : l'eau profonde arrête cavaliers et engins ; à pied, très lente
  (fatigue, moral, noyade selon l'armure). Un ordre de marche qui franchit la rivière passe par le
  passage le moins coûteux. Un régiment plus large que le tablier défile lentement (goulot) et combat
  mal depuis le pont ; celui qui tient la tête de pont frappe plus fort ; l'assaillant d'un gué ou de
  l'eau profonde est pénalisé. Marche en colonne sur la route plus rapide (×1,3).
- **IA** (`ai.rs`, `relief_ai.rs`) : un défenseur séparé de l'ennemi par la rivière et pas nettement
  plus fort tient la berge au passage que l'ennemi prendrait (tireurs en retrait de la berge, ligne en
  tête de pont). L'attaquant choisit son passage (marche, relief, largeur à défiler, tireurs ennemis
  qui couvrent la sortie), attend un temps sur sa berge pendant le duel de tir, puis franchit et se
  reforme au-delà. Pas de charge dans l'eau, sur un pont ni contre une berge escarpée.
- **Dimensions** : toute position se lit sur le champ (`field.width`, `field.depth`,
  `hydro::battle_lines`, `ai::deployment_center(field, side)`), jamais sur 1200 × 800 ; à la fusion
  d'EP1 (ADR 0031), `battle_lines` cède la place à `Battlefield::attacker_line_z/defender_line_z`.
- **Données** : tous les nombres dans `data/rules/battle_water.json`, validé par
  `data/schemas/battle_water_rules.schema.json` (test pytest), embarqué à la compilation.
- **Rendu** (Godot) : rivière de largeur variable, ruisseaux, pierres des gués, ponts du kit Blender
  (travées et culées de pierre et de bois, `tools/blender_scripts/building_kit.py`), écume aux piles,
  routes de la simulation, minicarte.

## Conséquences

- Les passages deviennent l'enjeu des batailles fluviales ; EP6 (villages, moulins à eau) et EP7
  (Crécy, Poitiers, Azincourt) s'appuient sur ce réseau.
- Le champ reste une grille de 10 m ; l'itinéraire ne cherche que parmi les passages de la rivière
  principale (les ruisseaux se traversent en ralentissant) : pas d'A* général.
- Nouvelles réglages à équilibrer avec la sonde IA contre IA
  (`cargo test --release -p sim-battle --test ep3_probe -- --ignored --nocapture`).

## Cohabitation avec R4 (ADR 0046)

À la fusion de main dans `integration/epic`, l'IA de position de R4 (`defensive_ground` : score de
position, crête militaire, contre-pente, couverts) et la tenue de berge d'EP3 (`river_hold`)
choisissent toutes deux le terrain d'un camp défensif. Priorité retenue dans `plan_field` :

1. **La berge d'abord** quand la rivière sépare le défenseur de l'ennemi et que le passage que
   l'ennemi prendrait est tenable, c'est-à-dire à moins de `RIVER_REACH` (420 m) de la ligne de
   déploiement (`river_hold` rend alors un couvert `CoverKind::River`) : une rivière à franchir sous
   les traits vaut plus que n'importe quelle hauteur, et c'est l'enjeu même des batailles fluviales.
2. **Sinon la position R4** (`defensive_ground`) : hauteur, glacis, couverts, crête militaire.

Les déclencheurs de la posture défensive s'additionnent : le défenseur se met sur la défensive s'il
est plus faible, tient des hauteurs (R2b), a une rivière devant lui (EP3) ou reçoit un ennemi qui
marche sur lui (R4). L'attaquant qui attend sur ses hauteurs face aux arcs (R4) ne cherche pas de
passage ; un camp qui avance choisit toujours son passage (EP3). Les tireurs d'une berge n'ont pas de
crête militaire (le couvert fixe leur poste) ; ceux d'une crête nue gardent celle de R4.
