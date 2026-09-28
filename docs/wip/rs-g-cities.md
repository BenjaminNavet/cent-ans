# RS-G — villes emblématiques v2 (VH8) et fleuves

Chantier RS (restes des wip). Branche `feat/rs-g-cities` (worktree d'agent, depuis `main` deff9c42).
Contexte : `docs/wip/sz-suites-zoom.md` (« Suites non lancées »), `docs/wip/vh-villes-historiques.md`,
format `docs/landmarks-v2.md`. Liens symboliques non versionnés vers le dépôt principal :
`data/map/pyramid`, `tools/geo/raw` (**ne rien cuire dedans** : c'est le cache du dépôt principal).
Dylib : cible cargo privée `core/target-rs-g` (à supprimer à la fin).

## Lots
| # | Contenu | État |
|---|---|---|
| 1a | Bordeaux v2 vers 1340 (`data/landmarks_v2/bordeaux.json`) | agent en cours (`docs/wip/rs-g-bordeaux.md`) |
| 1b | Avignon v2 vers 1340 | agent en cours (`docs/wip/rs-g-avignon.md`) |
| 1c | Calais v2 vers 1340 | agent en cours (`docs/wip/rs-g-calais.md`) |
| 1d | Bruges v2 vers 1340 | agent en cours (`docs/wip/rs-g-bruges.md`) |
| 1e | Relecture historienne des quatre villes (`docs/histoire/relecture-vh-*.md`) | à faire |
| 2 | Paris : monuments fusionnés par cellule de 600 m (`TownBuilder._extra_meshes`, `merge_monuments`, shader `base_source` 2 : base et ancrage par sommet dans `CUSTOM0`) | code fait, test Godot à passer |
| 3a | Bande sèche le long des rives : `river_widths.json` Rouen 260 m, Orléans 440 m, Tours 460 m (+ ancrages voisins) | données faites ; recuisson `hydro-fine` par le joueur |
| 3b | Couloir de fleuve `towns_1340.json` : `towns.pick_river_feature` (un fleuve ≥ 4 × plus large que le ruisseau le plus proche l'emporte) ; Tours 15 → 400 m, 19 autres villes corrigées (Nantes, Nevers, Bayonne…) ; Orléans avait déjà la Loire | fait, `geo towns` relancé (5 min) |
| 3c | Bosses GLO-30 de la Cité de Londres : E3/E4 gardaient le modèle de surface (tours de 25-50 m) sous les zones de détail ; `detail_dem.apply_parent_updates` cascade les moyennes 2 × 2 jusqu'à E3, `BAKE_VERSION` 6 | code fait ; recuisson par le joueur |
| 4 | Tests pytest, Godot (vh4, smoke), fusion de `main` | à faire |

## Mesures (scripts de travail, non versionnés)
- Chenal du relief (cellules ≤ eau + 0,6 m le long de la normale) contre demi-largeur du ruban :
  Orléans (E7) p90 jusqu'à 260-324 m d'un côté, 30-180 m de l'autre, ruban 175 m ; Tours (E4) p90
  jusqu'à 224-333 m, ruban 200 m ; Rouen île Lacroix (E7) p90 130-193 m, ruban 100 m.
- Cité de Londres (E3-E7, résidu à la médiane 120 m) : E3 max 24,7 m, E4 max 51,9 m (GLO-30,
  immeubles), E5-E7 max 4-8 m (LiDAR EA).

## Commandes à lancer par le joueur (recuisson, non faite ici)
Voir la section finale de ce fichier (mise à jour à la fin du lot).

## Journal
- 28/09 : squelette ; quatre agents villes lancés ; lots 2, 3a-3c codés.

## Prochaine étape
Tests Godot du lot 2 (dylib en cours de compilation), puis attente des villes, relecture.
