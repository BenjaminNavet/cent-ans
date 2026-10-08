# Échelle « Total War » : colonies prenables dans les provinces

Date : 2026-09-24. Statut : validé par Benjamin (approche B). Voir ADR 0005.

## 1. Constat et objectif

La v1 ressemble trop à Crusader Kings : 132 provinces, **une seule ville par province**, relief à
719 m/px, zoom qui s'arrête à l'échelle d'une région. Le joueur veut un jeu **à la Total War** :
une vue qui descend jusqu'au comté, avec beaucoup plus de lieux habités, de places fortes et de détail.

Décisions prises avec le joueur :

| Sujet | Décision |
|---|---|
| Emprise | Inchangée (Europe de l'Ouest élargie, 132 provinces, mêmes frontières) |
| Colonies | **Prenables séparément** : chaque colonie a propriétaire, contrôleur, garnison, siège, bâtiments |
| Structure | **B** : la province reste l'unité territoriale et contient 3 à 6 colonies |
| Zoom rapproché | Vue « comté » (30-50 km à l'écran) : maquettes de colonies, hameaux, routes, champs, armées en figurines |
| Déplacement | Sur le graphe des colonies (pas de mouvement continu, reporté) |

Hors périmètre : mouvement continu des armées, vue « paysage » (haies, moulins individuels),
changement d'emprise ou des frontières de provinces.

## 2. Principe de partage province / colonie

**La terre et les gens restent à la province ; les pierres et les hommes d'armes passent à la colonie.**

| Province (`ProvinceState`, conservé) | Colonie (`SettlementState`, nouveau) |
|---|---|
| population par classes, révolte, `revolt_seasons` | `owner` (de jure), `controller` (occupant) |
| dévastation (chevauchée) | `garrison`, `siege` |
| hérésie, régime alimentaire (`diet`) | `buildings`, `construction`, `recruit_queue` |
| ressources, terrain, climat (données statiques) | niveau de fortification |

Champs **retirés** de `ProvinceState` : `garrison`, `siege`, `recruit_queue`, `buildings`,
`construction`. `owner` et `controller` de la province deviennent **dérivés** :
propriétaire et contrôleur de sa **cité** (colonie capitale). Des accesseurs
`province_owner(id)` / `province_controller(id)` remplacent la lecture directe des champs.

## 3. Données

### 3.1 Colonies : `data/settlements/<province_id>.json`

Un fichier par province (un agent peut traiter une région sans conflit), contenant un tableau
validé par `data/schemas/settlement.schema.json` :

```json
{
  "id": "set_agen",
  "province": "prov_agenais",
  "kind": "city",
  "name": { "display": "Agen", "local": "Agen", "local_language": "occitan" },
  "lonlat": [0.6163, 44.2049],
  "weight": 40,
  "owner": null,
  "fortification_level": 2,
  "buildings": ["bld_market", "bld_stone_walls"],
  "port": false,
  "description": "…",
  "sources": ["Agen"]
}
```

- `kind` ∈ `city` (cité, exactement une par province, = l'actuel `capital_city`),
  `town` (ville murée ou bastide), `castle` (château, forteresse), `abbey` (abbaye, commanderie),
  `village` (bourg ou village non fortifié).
- `weight` (1-100) : part de la province rattachée à la colonie (impôt, recrutement). La somme par
  province est normalisée au chargement ; pas besoin qu'elle fasse 100.
- `owner` : `null` = propriétaire de la province en 1337. Renseigné pour les enclaves historiques
  (châteaux anglais en Agenais, places navarraises en Normandie, etc.).
- `fortification_level` : 0 pour `village`, sinon 1-4 ; remplace `Province.fortification_level`
  (qui reste lu pour initialiser la cité si absent).
- `port` : la colonie sert de port pour les traversées maritimes.
- Les bâtiments de départ de la province (`Province.buildings`) sont **attribués à la cité** ;
  les autres colonies reçoivent leurs bâtiments propres (fichier ci-dessus).
- Règles de validation (test Python + chargement Rust) : 3 à 6 colonies par province, exactement une
  `city` dont le nom correspond à `capital_city`, coordonnées dans la province (lecture de
  `province_ids.png`), ids uniques, au moins une colonie `port` par province côtière.

Contenu : ~550 colonies **historiquement attestées en 1337**, nom d'époque en langue locale,
sources citées (même exigence que `docs/design/provinces-1337.md`). Recherche faite par agents,
région par région.

### 3.2 Bâtiments

`building.schema.json` gagne `settlement_kinds` (liste des types de colonie autorisés, défaut : tous
sauf `village`). Exemples : murailles et donjon interdits aux villages, marché et guildes réservés
aux `city`/`town`, scriptorium aux `abbey`.

### 3.3 Hameaux (décor) : `data/map/hamlets.json`

~3 000 lieux habités réels extraits de GeoNames (CC BY 4.0, classes `PPL*`), filtrés par densité
(espacement minimal ~6 km), en coordonnées carte, avec `province` et nom. **Pas d'état de jeu** : ils
sont rendus comme maquettes et apparaissent brûlés selon la dévastation de leur province.
Attribution de GeoNames ajoutée à `docs/geo.md`.

### 3.4 Routes : `data/map/roads.geojson`

Voies romaines d'Itiner-e (CC BY 4.0), reprojetées. Si le jeu de données n'est pas téléchargeable
sans compte, repli : routes calculées par plus court chemin de coût (relief, fleuves) entre colonies
voisines. Chaque arête du graphe (§ 4.4) porte `road: bool`.

## 4. Cœur Rust

### 4.1 Types (`data-model`)

- `SettlementId` (`set_*`), `Settlement` (données statiques § 3.1), `SettlementKind`.
- `GameData.settlements: BTreeMap<SettlementId, Settlement>` et index
  `settlements_by_province`.
- `GameData.settlement_graph` : arêtes `(from, to, cost, road, sea)` chargées depuis
  `data/map/settlement_graph.json` (produit par `tools/geo`, § 5). Format (lot C1) :
  `{"edges": [{"from": "set_rouen", "to": "set_caen", "cost": 3.2, "road": true, "sea": false}]}`.
  `road` et `sea` valent `false` par défaut ; une arête est **non orientée** (une seule entrée par
  paire) ; `cost` est un réel ≥ 0 dans l'unité des points de mouvement. Une arête vers une colonie
  inconnue est ignorée avec un avertissement. Fichier optionnel : absent, le graphe est vide.
- `GameData.settlement_rules` : `data/settlements/rules.json` (schéma
  `settlement_rules.schema.json`) : `starting_garrison` par type de colonie, `full_province_bonus`
  (`income_percent`, `unrest_per_season`).
- **Repli (lot C1)** : toute province sans fichier, ou dont le fichier n'a pas de `city`, reçoit au
  chargement une cité générée depuis `capital_city` : id `set_<slug ASCII du nom affiché>`
  (`set_<slug>_<province>` en cas de collision), `weight` 100, fortification et bâtiments de la
  province, `port` si la province est côtière avec ports. Les incohérences des fichiers (province ou
  faction inconnue, bâtiment inconnu, id en double, nombre de colonies hors 1-6, plusieurs cités)
  produisent des avertissements, jamais un échec ; seul un JSON invalide est fatal.

### 4.2 État (`sim-campaign/state.rs`)

```rust
pub struct SettlementState {
    pub owner: FactionId,
    pub controller: FactionId,
    pub garrison: Vec<Unit>,
    pub siege: Option<SiegeState>,
    pub buildings: Vec<BuildingId>,
    pub construction: Option<Construction>,
    pub recruit_queue: Vec<UnitTypeId>,
    pub fortification_level: u8,
}
```

`CampaignState.settlements: BTreeMap<SettlementId, SettlementState>`.
`STATE_VERSION` passe de 4 à **5** ; une sauvegarde v4 est refusée avec un message explicite
(« sauvegarde d'une version antérieure à la refonte des colonies »), pas de migration.

### 4.3 Règles

- **Contrôle de province** : contrôleur de la cité. Tenir **toutes** les colonies d'une province
  donne +10 % de revenu et −5 de révolte par saison (valeurs dans `data/`, pas en dur).
- **Revenu** : l'impôt calculé pour la province est réparti entre les contrôleurs de ses colonies
  au prorata de `weight`. Une colonie assiégée ne rapporte rien ce tour.
- **Ressources** (`FactionState.goods`) : comptées pour le contrôleur de la cité.
- **Recrutement** : se fait dans une colonie ; les bâtiments de la colonie fixent les unités
  disponibles, la population de la province fournit les hommes (classes inchangées), plafonné par
  `weight`.
- **Construction** : une à la fois **par colonie**.
- **Siège** : logique M8 inchangée, appliquée à la colonie. Un `village` sans garnison est pris dès
  l'arrivée ; avec garnison, bataille puis prise. Les assauts 3D utilisent le type de colonie pour
  choisir la maquette de fortification.
- **Chevauchée** : ordre donné par une armée présente dans une province ennemie ; agit sur la
  dévastation de la province comme aujourd'hui.
- **Révolte** : les rebelles apparaissent à la cité.
- **Victoire / objectifs** : les conditions exprimées en provinces restent valides (contrôle de
  province dérivé) ; celles qui citent une ville visent sa colonie.
- Les valeurs de répartition (garnisons de départ par type, bonus de province complète) vivent dans
  `data/settlements/rules.json`.

### 4.4 Déplacement (`movement.rs`)

- `Army.location: SettlementId` ; `Army.path: Vec<SettlementId>`.
- Graphe : à l'intérieur d'une province, triangulation de Delaunay des colonies (arêtes trop longues
  retirées) ; entre provinces voisines, les 1 à 3 paires de colonies les plus proches ; liaisons
  maritimes entre colonies `port` des provinces `sea_neighbors`.
- Coût d'arête = distance × coût du terrain dominant de la province, divisé par 2 sur route.
  Points de mouvement rééchelonnés pour qu'une armée parcoure la même distance par saison qu'en v1.
- Entrer sur une colonie ennemie arrête le mouvement (siège ou prise). Deux armées ennemies sur le
  même nœud se battent (règle actuelle, appliquée au nœud).
- Dijkstra sur ~550 nœuds : coût négligeable, aucune structure particulière.

### 4.5 IA (`ai/`)

Les objectifs deviennent des colonies : prendre la cité donne la province, les autres colonies sont
des cibles secondaires pondérées par `weight` et fortification. Garnisons réparties par menace.

### 4.6 Pont Godot

Getters ajoutés : `settlements()` (id, province, kind, nom, position carte, contrôleur, siège),
`settlement_detail(id)`, `province_settlements(province)`. Les ordres de recrutement et de
construction prennent un `SettlementId`. Les anciens getters par province restent, en lecture
dérivée.

## 5. Pipeline géo (`tools/cent_ans_tools/geo`)

- **Relief 8192²** (≈ 360 m/px) depuis ETOPO 15″ (résolution suffisante), découpé en
  **16 × 16 tuiles** PNG 16 bits de 512² dans `data/map/height/`. `map.json` gagne
  `height_tiles`. `province_ids.png`, `land_mask.png` et `splat.png` restent à 4096² (picking et
  masques suffisent).
- `settlements` : projette les colonies, vérifie qu'elles tombent dans leur province, construit
  `settlement_graph.json`, produit `docs/img/settlements-preview.png`.
- `hamlets` : GeoNames → `hamlets.json`.
- `roads` : Itiner-e (ou repli) → `roads.geojson`.
- Tout reste reproductible en une commande (`cent-ans geo build`), téléchargements en cache.
- Formats précisés par le lot C3 (détail dans `docs/geo.md`) :
  - `data/map/settlements_px.json` : `{"set_…": [x, y]}`, position de jeu en pixels carte 4096
    (ramenée dans la province si le `lonlat` tombe dehors) ;
  - `settlement_graph.json` : coût sans unité = km × terrain (2 montagnes/marais, 1 sinon ;
    moyenne des deux provinces à une frontière) ÷ 2 sur route ; arête `sea` = 100 + km ; une
    arête frontalière à plus de 50 % sur l'eau entre deux ports devient `sea` ;
  - `roads.geojson` : `LineString` en pixels carte, propriétés `name`, `type`
    (`main`/`secondary`/`computed`), `certainty`, `source` (`itiner-e`/`computed`) ; Itiner-e
    complété par des routes calculées hors de l'Empire romain ;
  - `hamlets.json` : `[{"name", "px": [x, y], "province"}]` ;
  - `map.json` : `"height_tiles": {"size_px": 8192, "tile_px": 512, "dir": "height",
    "pattern": "h_{col}_{row}.png"}`, tuiles sans recouvrement.

## 6. Rendu Godot

Trois paliers selon la distance caméra (seuils dans une ressource de réglages) :

| Palier | Visible |
|---|---|
| **Loin** | provinces colorées par contrôleur (hachurées si partagées), noms de provinces |
| **Moyen** | icônes de colonies (type + couleur du contrôleur), noms des cités, routes principales |
| **Près** (vue comté) | maquettes 3D par type de colonie, hameaux en `MultiMesh`, routes, champs et forêts procéduraux, noms de toutes les colonies, armées en figurines |

- Terrain : tuiles chargées à la demande près de la caméra (8192²), 4096² au loin.
- `campaign_camera.gd` : `min_distance` abaissée pour la vue comté ; tangage déjà lié au zoom.
- Maquettes : générateurs Blender existants (`m10-assets`) étendus aux 5 types de colonie.
- Picking : clic sur une maquette ou une icône = colonie ; ailleurs = province.
- Panneau de province : onglet **Colonies** (liste, contrôleur, garnison, siège). Panneau de colonie :
  garnison, bâtiments, recrutement, construction (repris des onglets actuels de province).
- Chemins d'armée : le long des arêtes du graphe (routes visibles).

## 7. Tests

- Rust : répartition du revenu par poids, contrôle dérivé de la cité, bonus de province complète,
  prise immédiate d'un village sans garnison, siège d'un château, pathfinding sur le graphe des
  colonies (route moins chère), refus d'une sauvegarde v4, déterminisme conservé.
- Python : validation des fichiers de colonies (§ 3.1), graphe connexe (chaque province atteignable),
  chaque colonie dans sa province.
- Godot : smoke test 10 tours ; test de 50 tours IA contre IA sans erreur, temps de tour mesuré.
- Captures des trois paliers de zoom dans `docs/img/colonies/`.

## 8. Lots (vagues de 3 agents au plus)

| Lot | Contenu | Dépend de |
|---|---|---|
| C1 | Schéma `settlement`, types `data-model`, squelette `SettlementState`, tests désactivés ; génération automatique d'une cité par province depuis `capital_city` (le jeu reste jouable) | — |
| C2a-c | Recherche des colonies : (a) France du Nord + Flandre + Pays-Bas + Rhénanie, (b) France du Sud + Ibérie + Italie + Suisse, (c) Îles britanniques + Bretagne + Normandie | C1 |
| C3 | Pipeline géo : graphe des colonies, relief 8192² en tuiles, hameaux, routes | C1 (graphe finalisé après C2) |
| C4 | Refonte du cœur : état, contrôle dérivé, économie, recrutement, construction, siège, mouvement, sauvegarde v5 | C1, C3 (graphe) |
| C5 | Pont + UI : panneaux de colonie, ordres par colonie | C4 |
| C6 | Rendu par paliers : icônes, maquettes, hameaux, routes, tuiles de relief, caméra | C3, C5 |
| C7 | IA sur les colonies, équilibrage, test 50 tours, documentation (`manuel.md`, codex) | C4-C6 |

Suivi de l'orchestration : `docs/archive/chantiers.md`. Coût cloud prévu : 0 $.
