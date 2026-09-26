# ADR 0078 — Villes emblématiques à l'échelle 1:1 géoréférencées (format `landmark` v2)

Date : 2026-09-26. Statut : accepté. Chantier VH (`docs/wip/vh-villes-historiques.md`), lots VH0
et VH4 (`docs/wip/vh4-villes-emblematiques.md`), sous l'orchestration SZ
(`docs/wip/sz-suites-zoom.md`, défaut S3 de ZG7c). Complète l'ADR 0015 (loupe radiale), l'ADR
0036 (carte zoomable, addendum « villes emblématiques ») et les ADR 0021/0047 (kit de bâtiments).
Numéro réservé 0037 à l'origine, pris depuis par la difficulté.

## Contexte

Les sept villes emblématiques (Paris, Londres, Rouen, Bordeaux, Avignon, Calais, Bruges) sont des
maquettes L1/L2 : un plan en mètres (`data/landmarks/<id>.json`) déformé par une loupe radiale
(×3,5 au centre, ADR 0015), cuit dans Blender en un glTF drapé sur le relief. Depuis ZG
(ADR 0036), la caméra descend jusqu'à 200 m sur un relief fin à 3-25 m. La maquette y est fausse :
à Rouen, la Seine agrandie ne tombe pas dans la vraie vallée, une falaise traverse la ville, le
plan d'eau se dresse à la verticale (recette ZG7c, défaut S3). ZG4b a posé un plancher de caméra
provisoire (`landmark_min_distance`, 2,6 unités ≈ 1,9 km) au-dessus de ces villes.

Les villes ordinaires, elles, sont déjà à l'échelle réelle depuis ZG6 (`TownPlan`,
`TownBuilder`, `TownLayer`) : plan procédural sur le relief fin, parcelles en lanières, kit bas
détail, HLOD par maison, hauteurs posées par le shader à la hauteur affichée ZG8.

## Options

- **A. Garder la maquette et la « dégonfler » au zoom** (loupe → 1) : le plan de la maquette reste
  schématique (13 rues pour Paris) et ses bâtiments sont cuits dans Blender pour la loupe.
- **B. Un format v2 géoréférencé rendu par le moteur ZG6 étendu** : la ville est décrite par ses
  éléments vrais (rues, enceintes, portes, quais, ponts, monuments à gabarit réel, quartiers),
  en coordonnées EPSG:3035, sans loupe ; le tissu (parcelles, maisons) est généré à l'exécution
  sur le relief fin comme pour les villes ordinaires.
- **C. Villes entièrement cuites hors ligne** (Blender, glTF 1:1 par ville) : le relief fin est
  streamé et exagéré à l'exécution (ZG8), une ville cuite ne suivrait pas la hauteur affichée.

## Décision

**Option B.** La maquette L1/L2 reste l'image de la ville en vue stratégique ; la ville 1:1 la
remplace au zoom rapproché, avec un fondu.

1. **Format `landmark` v2** : `data/landmarks_v2/<id>.json`, schéma
   `data/schemas/landmark_v2.schema.json`. Un fichier par ville, lié à la maquette v1 (`landmark`)
   et à sa colonie (`settlement`). Coordonnées **EPSG:3035** (comme le reste de la carte fine) :
   origine `origin_3035` [E, N] en mètres, géométrie en décalages [dE, dN] en mètres **dans les axes
   de la grille EPSG:3035** (translation seule : ni rotation, ni échelle, ni loupe). Contenu :
   enceintes datées (portes, tours, fossé), rues hiérarchisées (principale, secondaire, venelle),
   quais, ponts (maisons, chapelle, portes, moulins), monuments à gabarit réel paramétré, quartiers
   (densité, mélange de maisons, toitures), espaces libres (places, cimetières, jardins, prés),
   paramètres du parcellaire, sources et licences. Les tracés de rues hérités peuvent être tirés
   d'OpenStreetMap (ODbL, crédit obligatoire) par l'outil `cent-ans geo landmarks` ; le fleuve est
   celui de la carte fine (`rivers_fine`), recopié par le même outil, pour que les quais tombent
   sur l'eau affichée.
2. **Moteur de rendu (VH4)** : `LandmarkPlan` (plan pur, fil de travail) produit un plan au format
   de `TownPlan` (rues drapées, parcelles en lanières de 5-8 m × 20-40 m le long des vraies rues,
   maison sur rue et cour ou jardin derrière, enceinte polygonale, tours, portes, pont) plus les
   monuments à gabarit réel (`LandmarkMonuments`) ; `TownBuilder` le construit par étapes
   (ADR 0051), maisons du kit bas détail BR1 de près, blocs au-delà (HLOD par maison dans le shader),
   nœuds par cellule d'îlots ; `LandmarkCityLayer` gère le streaming et le fondu. Hauteurs en
   mètres, posées par `town_building.gdshader` à la hauteur affichée ZG8 (même source que le
   terrain). Rendu seulement : rien dans `core/`.
3. **Transition** : la maquette sous loupe reste visible tant que le palier vallée n'est pas
   atteint ; entre les poids vallée 0,35 et 0,65, elle se dissout (tramage) au-dessus de la ville
   1:1 déjà construite ; au-delà, seule la ville 1:1 est affichée.
4. **Caméra** : le plancher ZG4b (`landmark_min_distance`) ne s'applique plus aux villes qui ont
   un fichier v2 ; il reste pour les autres jusqu'à leur migration.
5. **Villes** : Rouen vers 1340 prouve le format (VH4). Paris, Londres et Orléans suivent (VH5-VH7),
   puis Bordeaux, Avignon, Calais et Bruges (VH8). Le format est documenté dans
   `docs/landmarks-v2.md`.

## Conséquences

- Les villes emblématiques sont posées dans la vraie vallée : les rues suivent le relief fin, les
  quais le fleuve affiché, les monuments ont leur gabarit réel (cathédrale de Rouen : 137 m).
- Une ville v2 est une donnée texte (≈ 140 ko pour Rouen, rues OSM comprises) ; le tissu est
  régénéré à chaque chargement (déterministe, graine de la ville), comme les villes ordinaires.
- La maquette L1/L2 et le décor de siège L3 restent inchangés (le fichier v1 garde la loupe).
- Les sources non commerciales (Cassini-Geopeuple, Open Domesday, carte d'Agas/MoEML, scans
  Gallica) ne servent qu'au contrôle humain, jamais à une extraction. OSM est cité dans
  `sources` de chaque ville et dans `docs/credits.md`.
- Limite : le parcellaire est généré le long des rues (pas de cadastre réel) ; Paris pourra
  importer le parcellaire ALPAGE (ODbL) dans un champ `parcels` réservé par le schéma (VH5).

## Addendum VH7 (2026-09-26) : ville 1:1 sans maquette

Orléans n'a ni maquette L1/L2 ni bloc de siège : c'était une colonie ordinaire (ville ZG6). Plutôt
que de cuire une petite maquette dans Blender, le champ `landmark` devient facultatif : sans lui, la
colonie garde en vue stratégique sa maquette de colonie ordinaire, et la ville v2 remplace la ville
ZG6 au zoom rapproché, affichée au même poids vallée que les villes ZG6 (pas de fondu, pas de
plancher ZG4b). Une maquette L1/L2 et un bloc `siege.battle` (ADR 0026) pourront être ajoutés plus
tard sans changer le fichier v2.
