# 0242 — Quatorze biomes

Statut : accepté (10-09, chantier TX, tranche 2a)

## Contexte
`data/map/biomes.png` ne portait que 7 classes terrestres (ADR 0143). Les textures régionales du
chantier TX (sols de campagne et de bataille, végétation, bâtiments) demandent une finesse
régionale : un désert n'est pas un semi-aride, la toundra n'est pas la taïga, la plaine de Hongrie
n'est pas le Bassin parisien. Beaucoup de consommateurs indexent un tableau par biome (essences,
sol, palette de la carte de couleur, oiseaux, rochers, peuplements) ; les 15 classes ne doivent
casser aucun d'eux.

## Décision
- Indices 0-7 figés. Sept sous-classes 8-14, chacune avec un `parent` (une classe de base) dans
  `data/map/biomes.yaml` : 8 désert (←7), 9 toundra (←5), 10 continental est (←2), 11 Atlantique
  sud (←1), 12 pannonien (←2), 13 hémiboréal (←5), 14 maquis égéen (←3).
- La cuisson (`cent-ans geo biomes`, `refine` dans `tools/cent_ans_tools/geo/biomes.py`) taille les
  sous-classes dans leurs parents après les règles de l'ADR 0143 et avant le lissage : Köppen BW
  pour le désert, ET ou taïga au nord de 63° / 68,5° pour la toundra (pas les Alpes), boîtes
  lon/lat et plafond d'altitude pour hémiboréal, pannonien, Atlantique sud, maquis égéen (Csa),
  lon >= 24° E pour le continental est. Un bruit basse fréquence décale lon/lat (`edge_noise`) pour
  que les lisières ne soient pas des méridiens. Tous les seuils sont dans `biomes.yaml`
  (schéma `biomes.schema.json`).
- **Table des parents unique** : la cuisson écrit `data/map/biome_parents.json` (dérivée de
  `biomes.yaml`, test de synchronisation) ; GDScript la lit (`BiomeParents`), Python la calcule
  (`biomes.parents`), le Rust la reçoit dans les tables que GDScript lui passe (`biome_parent`).
  Aucune copie en dur.
- **Repli sur le parent** partout : une donnée qui ne mentionne pas un biome 8-14 hérite de son
  parent (`tree_species` : poids et paramètres, un poids 0.0 explicite coupe ; peuplements : un
  biome qu'aucun peuplement ne cite ; rochers et filtres de la couche rurale : masque de biomes ;
  oiseaux : noms d'habitat ; palette de la carte de couleur : poids du parent ; sol : ligne du
  parent). Côté Rust, `SpeciesTable::resolve_biome` et `StandTable::resolve_biome`.
- **Sol du terrain** (`hb_ground.gdshaderinc`) : les lignes 8-15 de la table appartiennent aux
  paysages agricoles ME8 ; les biomes 8-14 sont rangés aux lignes 16-22 (`BiomeParents.table_row`,
  hauteur 24). Les clés de `ground_biome_mix.json` restent les indices de biome ; `HbGround.mix_rows`
  les convertit en lignes.
- **Lecteurs qui ne connaissent que 1-7** (`FieldPlan`, propriété d'une autre session) : la
  cuisson écrit aussi `data/map/biomes_base.png` (mêmes pixels, sous-classes repliées sur leur
  parent) et `HbGround._load_biomes` le lit à la place de `biomes.png`, sans changer sa signature.
- Données : `ground_biome_mix.json` (entrées complètes 8-14, matières de `dn_fields.json`),
  `tree_species` (paramètres 8-14, poids adaptés : toundra bouleau/saule, désert peuplier de wadi,
  hémiboréal épicéa/bouleau/chêne, Atlantique sud pin maritime), `colormap_style.yaml` (palettes
  8-14 ; la carte de couleur n'est pas recuite par ce lot).

## Conséquences
- Un nouveau biome ne casse aucun consommateur : sans entrée, il se comporte comme son parent.
- Les sols de campagne (2b) et de bataille (2c) peuvent s'accrocher à 14 biomes avec héritage.
- `FieldPlan` voit encore les parents (sol et cultures du parent) tant que la session DN ne lui
  apprend pas les lignes 16-22 ; les entrées complètes existent déjà pour ce jour.
- La carte de couleur et les tuiles de campagne n'utilisent les palettes 8-14 qu'à la prochaine
  recuisson de `cent-ans geo colormap`.
- Le désert couvre 16 % des terres de la carte (Afrique du Nord) ; part du continental est 15 %.
- Les lisières de boîtes restent anguleuses malgré le bruit : à affiner par des masques plus fins
  si le rendu des sols le demande.
