# Mouvement libre des armées (façon Total War)

Date : 2026-09-24. Statut : validé par le joueur. Complète `2026-09-24-echelle-colonies.md` et remplace son choix « déplacement sur le graphe des colonies » (ADR 0006).

## 1. Objectif

Les armées se déplacent librement sur la terre, comme dans la carte de campagne de Total War. Pendant son tour, le joueur bouge une armée : elle avance aussitôt, s'arrête où il veut dans la limite de ses points de mouvement, et peut attaquer ou assiéger dans la foulée. Les factions IA jouent ensuite une par une.

| Sujet | Choix |
|---|---|
| Position d'une armée | point libre sur la carte (pixels carte 4096, flottants), ou dans une colonie |
| Chemins | A* sur une grille de navigation de 2048² (≈ 1,44 km par case) produite par le pipeline géo |
| Tour | séquentiel : le joueur, puis chaque IA, avec exécution immédiate des mouvements ; ensuite les phases de fin de tour actuelles |
| Contact | zone de contrôle autour de chaque armée, bataille immédiate sur attaque |
| Mer | traversées de port à port inchangées (pas de flottes libres) |
| Vision | rayon autour des armées et des places |
| Sauvegarde | `STATE_VERSION` 6, versions antérieures refusées |

Hors périmètre : flottes libres, ravitaillement et attrition par case, embuscades, marches forcées, animation des batailles sur la carte de campagne.

## 2. Grille de navigation (lot M1, `tools/cent_ans_tools/geo/navgrid.py`)

- `data/map/navgrid.png`, 2048², 8 bits, **une valeur de coût par case**, avec 255 = infranchissable. Chaque case couvre 2 × 2 pixels carte, soit environ 1,44 km.
- Coût de base (sans unité, 10 = plaine) :
  - plaine et champs 10 ; collines 15 ; forêt 18 ; marais 25 ; montagne 30 ;
  - pente au-delà d'un seuil : infranchissable (cols gardés ouverts, voir plus bas) ;
  - mer et lacs : 255 ;
  - route de `roads.geojson` : coût × `road_cost_factor` (0,75, reprise de C7a), rastérisée sur 1 case de large.
- **Fleuves** : les grands fleuves de `rivers.geojson` (Loire, Seine, Rhône, Garonne, Rhin, Tamise, Meuse, Escaut, Dordogne, Saône, Pô, Tage, Èbre, Danube) sont infranchissables, sauf aux **passages** : ponts et gués historiques listés dans `data/map/crossings.json` (validé par un schéma, sourcé), et intersections route/fleuve d'Itiner-e. Tous les autres cours d'eau coûtent +10.
- **Cols** : les cols alpins et pyrénéens connus (Mont-Cenis, Grand-Saint-Bernard, Simplon, Somport, Roncevaux, Perthus…) sont forcés franchissables le long de leur route, via `crossings.json`.
- **Colonies** : la case de chaque colonie est franchissable, avec un coût de plaine.
- Contrôles du pipeline :
  - chaque colonie est reliée par la grille à toutes les colonies de sa masse terrestre ;
  - les îles sans port sont signalées ;
  - la carte de coûts est exportée en aperçu dans `docs/img/navgrid-preview.png`.
- `map.json` gagne `"navgrid": {"size_px": 2048, "file": "navgrid.png", "scale": 2}`.

## 3. Cœur (lot M2, `sim-campaign`)

### 3.1 Position

```rust
pub enum ArmyPosition {
    Field { x: f32, y: f32 },   // pixels carte 4096
    Settlement(SettlementId),   // stationnée, en garnison ou en siège
}
```

- `Army.position` remplace `Army.location`. Deux accesseurs dérivés :
  - `army_province(&Army)`, qui lit `province_ids.png` pour une armée en campagne ;
  - `army_point(&Army)`, qui renvoie la position de la colonie pour une armée stationnée.
- Le moteur lit `province_ids.png` et la grille une seule fois, dans `GameData`.
- `Army` gagne `movement_left: u32` (points restants ce tour) et `planned_path: Vec<Cell>` (reste d'un trajet sur plusieurs tours).

### 3.2 Points de mouvement et chemins

- Allocation par tour : `season_points` = `points_per_step` × `movement_steps(season)` × `season_scale`, repris de C7a, ce qui donne environ 210 km de plaine en été. L'unité est convertie en coûts de grille : 10 par case de plaine de 1,44 km.
- Les modificateurs existants (compétences, technologies, traits de général) s'appliquent au budget, comme aujourd'hui.
- `find_path(data, army, target_point)` : A* sur la grille, avec une heuristique octile et des déplacements sur 8 voisins. Le résultat est lissé par ligne de vue (théta*) pour éviter les zigzags.
- `reachable_area(army)` : Dijkstra borné par `movement_left`. Il renvoie les cases atteignables ; le pont s'en sert pour dessiner la bulle.
- Si la cible est hors de portée, l'armée avance au maximum et garde le reste du chemin dans `planned_path`. Au tour suivant, elle continue automatiquement au début de son tour, sauf nouvel ordre.
- Traversée maritime : on marche jusqu'au port, puis un ordre `Embark { army, to_port }` coûte tout le mouvement du tour, comme les arêtes `sea` du graphe C3. L'armée débarque au port d'arrivée.

### 3.3 Exécution immédiate

- `apply_order(MoveArmy { army, target })` fait avancer l'armée tout de suite, case par case, en décomptant `movement_left`. Elle s'arrête :
  - à la cible ;
  - à épuisement des points ;
  - en entrant dans la **zone de contrôle** d'une armée ennemie (rayon `zoc_radius_km` = 8, dans `data/movement/rules.json`) ;
  - en arrivant au contact d'une colonie ennemie.
- La réponse de l'ordre donne le trajet effectivement parcouru, pour l'animation, et la raison de l'arrêt.
- **Attaquer une armée** : `Attack { army, target_army }` si la cible est à moins de `engage_radius_km` (5) après déplacement. La bataille est résolue tout de suite par le système actuel (`battle_request` : bataille tactique ou automatique au choix du joueur ; automatique entre IA). Aucun mouvement n'est possible après une bataille.
- **Colonie ennemie** : y entrer lance un siège (l'armée passe en `Settlement(id)` avec un siège) ; un village non fortifié est pris directement. L'assaut reste un ordre à part (`Assault`).
- **Entrer dans une colonie amie** : l'armée y stationne. Elle peut laisser des unités en garnison (C7d) et en ressortir le même tour s'il lui reste des points.
- **Repli du perdant** (règle C7a, sur la grille) :
  - vers la place amie la plus proche à moins de `friendly_radius_steps` × 140 km de marche, sans traverser de zone de contrôle ennemie ;
  - sinon, recul de 15 km à l'opposé du vainqueur, avec les traînards de la règle « neutre » ;
  - sinon débandade.
- L'ordre des armées dans un tour n'a plus d'importance : chaque action est résolue au moment où elle est jouée.

### 3.4 Tour séquentiel

`end_turn` devient :

1. Le tour du joueur est clos.
2. Chaque faction IA, dans un ordre fixe (par id) :
   - elle poursuit les `planned_path` ;
   - son planificateur produit ses ordres ;
   - ils sont exécutés immédiatement, un par un.

   Les batailles IA contre joueur sont résolues automatiquement, avec un événement dans le rapport de saison. En v1, pas d'invitation à livrer une bataille tactique pendant le tour de l'IA.
3. Les phases actuelles de fin de tour (économie, sièges, population, événements, diplomatie…), dans l'ordre de la spec § 1.3, sans la phase de mouvement.
4. Les `movement_left` sont remis à neuf, puis c'est au tour du joueur, qui poursuit d'abord ses `planned_path`.

Le déterminisme est garanti par la graine, l'ordre des factions et l'ordre des ordres d'une IA.

### 3.5 Données

`data/movement/rules.json` et son schéma : `zoc_radius_km`, `engage_radius_km`, `retreat_fallback_km`, `vision_army_km`, `vision_settlement_km`, les coûts de terrain de la grille (lus aussi par le pipeline), `embark_cost` = tout le tour. Les réglages de portée de C7a restent dans `data/settlements/rules.json`.

## 4. IA (lot M3)

- Planification stratégique inchangée sur le graphe des colonies (C3/C7a) : choix de cibles, reprise des places, garnisons.
- L'exécution passe par la grille : l'armée vise le point de sa cible et `find_path` sur la grille fait le reste.
- Une IA attaque une armée ennemie dans sa bulle si le rapport de forces dépasse le seuil actuel, et évite les zones de contrôle d'armées plus fortes.
- Performance : moins de 50 ms par faction et par tour (A* borné, cache des chemins par tour).

## 5. Vision (lot M5, `vision.rs`)

Une case est vue si elle est à moins de `vision_army_km` (30) d'une armée alliée, ou de `vision_settlement_km` (20) d'une colonie contrôlée par soi ou un allié. Le brouillard de la session 6 bascule d'un masque par province à un masque par case, exporté au pont en texture 512² pour la minicarte et la carte.

## 6. Pont et interface (lot M4)

- Getters :
  - `get_reachable_area(army)` : image de la bulle en 2048², ou contour ;
  - `find_path_points(army, x, y)` : polyligne en pixels carte, avec le coût et le point d'arrêt de ce tour.
  - Les dictionnaires d'armée exposent `position` (x, y), `settlement`, `movement_left` et `planned_path`.
- Ordres : `move_army` avec une cible en point, `attack`, `embark`.
- Carte :
  - la **bulle** atteignable est dessinée au sol (shader sur une texture de masque) ;
  - le chemin est tracé en deux couleurs, ce tour puis les tours suivants ;
  - clic droit sur le sol pour déplacer, sur une armée ennemie pour attaquer, sur une colonie pour marcher dessus, assiéger ou stationner ;
  - l'armée est animée le long du trajet parcouru ;
  - le cercle de zone de contrôle est affiché au survol d'une armée ennemie.
- Les anneaux d'atteignabilité de C5 sont remplacés par la bulle. Le panneau de colonie et le bouton « Garnison » restent.

## 7. Tests

- Pipeline : les colonies d'une même masse terrestre sont connexes ; un fleuve n'est franchissable qu'aux passages ; les cols sont ouverts.
- Cœur :
  - un chemin contourne un fleuve jusqu'au pont ;
  - une armée s'arrête dans une zone de contrôle ennemie ;
  - un trajet sur plusieurs tours reprend au tour suivant ;
  - attaque à portée et hors de portée ;
  - siège en entrant dans une place ;
  - repli sur la grille ;
  - la sauvegarde v5 est refusée ;
  - une partie de 50 tours en IA contre IA sur 8 graines, avec les mesures de C7a (trésors, provinces, sièges, armées bloquées) dans la même bande qu'après C7a ;
  - performance du tour IA.
- Godot : clic au sol → déplacement immédiat → position mise à jour ; bulle affichée ; attaque ; smoke, `settlements_render_test`, `c5_settlements_ui_test` adapté.

## 8. Lots

| Lot | Contenu | Dépend de |
|---|---|---|
| M1 | Grille de navigation, `crossings.json` (ponts, gués, cols sourcés), aperçu | — |
| M2 | Cœur : position, A*, points, exécution immédiate, zone de contrôle, attaque, siège, repli, sauvegarde v6 | M1 (grille de repli uniforme pour les tests si M1 n'est pas prêt) |
| M3 | Tour séquentiel, IA sur la grille, performance | M2 |
| M4 | Pont + UI : bulle, clic au sol, chemin, animation | M2 |
| M5 | Vision par rayon, équilibrage sur 50 tours, `manuel.md`, codex | M3, M4 |

Suivi : `docs/wip/mouvement-libre.md`. Coût cloud prévu : 0 $.
