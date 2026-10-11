# 0343 — Déplacement : routes de province, fleuves navigables, cols et hiver, général

Statut : accepté (lot RG9 du chantier RG, retours de gestion).

## Contexte
Le déplacement libre (ADR 0006) repose sur une grille cuite `navgrid.png` : coûts de terrain, routes
antiques ×0,75, cols forcés au coût de la montagne, grands fleuves infranchissables sauf aux ponts
et gués (`crossings.json`). La grille est la même pour tous et ne change jamais : le joueur ne peut
rien faire pour se déplacer mieux, ni l'hiver ni le général ne comptent. Il veut que le
déplacement pèse davantage dans les choix.

## Décision
Tout ce qui dépend de l'état ou de l'armée s'applique **au moment du calcul du chemin**, sur une
grille *dérivée* (`NavGrid::derive` : mêmes dimensions, mêmes cases infranchissables, composantes
connexes partagées), sans jamais modifier la grille cuite. Les grilles dérivées sont mises en cache
par jeu de modificateurs (`GameData::derived_navgrid`, 6 au plus) ; `CampaignState::army_navgrid`
choisit la bonne pour une armée. Chemin, bulle d'atteinte, aperçu, marche réelle et IA passent par
elle : l'IA en profite sans code propre, et aperçu et coût réel ne divergent pas.

Toutes les valeurs sont des données : `data/movement/rules.json` (schéma `movement_rules`),
`data/movement/rivers.json` (schéma `navigable_rivers`), bâtiments et compétences.

1. **Route de province** : chaîne `bld_gravel_road` (Chemin empierré) puis `bld_royal_road`
   (Route royale), construite dans la cité ou la ville. Nouvel effet `road_movement` (−15 %, −30 %),
   pondéré comme les autres effets de province (`province_effect_percent`). Il multiplie le coût de
   toutes les cases de la province (via `province_ids.png`), par-dessus les routes cuites. Plancher du
   multiplicateur cumulé routes + col : `road_floor_factor` 0,5. L'IA construit ces bâtiments par sa
   logique générique.
2. **Cols** : les cases du couloir d'un `pass` de `crossings.json` (polyligne projetée, élargie d'une
   case, limitée aux cases de coût montagne) coûtent `pass_cost_factor` (0,7) du coût montagne.
3. **Hiver** : cases de montagne (coût de grille ≥ `winter_mountain_min_cost` = 30, la grille n'a pas de
   couche de terrain) et de col ×`winter_mountain_factor` (1,5), selon la saison de la campagne.
   Approximation assumée : un marais traversé par un cours d'eau (35) compte comme montagne.
4. **Général** : +`general_level_percent` (3 %) de points de mouvement par niveau au-dessus du premier,
   plafond `general_level_cap_percent` (30 %). Le pourcentage `movement` du général (Éclaireur) et ce
   bonus s'appliquent sur les points finaux, non sur les pas de la saison : +10 % de trois pas
   aurait été arrondi à zéro.
5. **Compétences** (branche commandement, rôle général) : `skill_eclaireur` (tier 1, `movement` +10 %),
   `skill_montagnard` (tier 2, nouvel effet `winter_movement` −100 % : plus de malus d'hiver),
   `skill_batelier` (tier 2, nouvel effet `river_movement` −25 %).
6. **Navigation fluviale** : choix retenu = un **ordre dédié `SailRiver`**, pas le mécanisme d'embarquement
   maritime ni des cases de fleuve franchissables.
   - Embarquement EM (ADR 0167) : il consomme toute la saison, procède par arêtes de mer du graphe de
     colonies, avec interception et tempêtes ; autant de règles qui n'ont pas de sens sur un fleuve.
   - Rendre franchissables les cases de fleuve ouvrirait la grille à pied à travers les grands fleuves
     (le fleuve n'est plus une barrière) et casserait les composantes connexes.
   - `data/movement/rivers.json` liste, pour chaque fleuve (Seine, Loire, Garonne, Dordogne, Rhône,
     Saône, Tamise, Rhin, Meuse, Escaut), ses places de la source à l'embouchure avec leur abscisse
     `km` le long du cours (mesurée sur `rivers.geojson`). Une armée dans une place riveraine peut
     rejoindre toute autre place du même fleuve (`CampaignState::river_voyages_all`) à condition de
     n'entrer ni passer dans une place ennemie ou tenue par une armée ennemie. Coût en points :
     distance fluviale convertie comme une route (`road_cost_factor`) × `river_downstream_factor`
     (0,5) ou `river_upstream_factor` (0,8) × (1 + `river_movement`), prélevé sur le mouvement restant ;
     il doit tenir dans le tour (`RiverTooFar` sinon). Pas de changement de fleuve en route.
   - Pont Godot : `get_river_voyages`, `sail_river` ; ordre `sail_river` en JSON.
7. **Interface** : l'infobulle d'aperçu de trajet détaille « Terrain · Routes · Cols · Hiver ·
   Général » (`PathPlan::breakdown`, `general_percent`) ; un clic droit sur une place riveraine
   atteignable par le fleuve, plus vite que par terre, propose le trajet fluvial (infobulle « ⛵ »).
   La vue province affiche le bâtiment Route comme les autres (liste des bâtiments). Pas de nouvel écran.

## Conséquences
- Les routes de province deviennent un investissement : −30 % de coût dans toute la province.
- L'hiver ralentit franchement les Alpes, les Pyrénées et leurs cols, sauf avec Montagnard.
- Le cache coûte jusqu'à 6 grilles de la taille de la grille cuite (octets, une par jeu de modificateurs).
- L'IA ne lance pas d'ordre `SailRiver` (elle profite des routes, des cols et du général par le
  calcul de chemin) ; point ouvert.
- Pas d'icône dédiée pour les deux bâtiments (le jeu en affiche une par défaut) ; à générer.
