# RS-G — villes emblématiques v2 (VH8) et fleuves

Chantier RS (restes des wip). Branche `feat/rs-g-cities` (worktree d'agent, depuis `main` deff9c42,
`main` fusionnée le 28/09 : 9fb15b63). Contexte : `docs/wip/sz-suites-zoom.md` (« Suites non
lancées »), `docs/wip/vh-villes-historiques.md`, format `docs/landmarks-v2.md`. Liens symboliques
non versionnés vers le dépôt principal : `data/map/pyramid`, `tools/geo/raw` (**rien n'y a été
cuit** : c'est le cache du dépôt principal). Cible cargo privée supprimée.

## État : terminé (à fusionner par l'orchestrateur)
| # | Contenu | État |
|---|---|---|
| 1a | Bordeaux vers 1340 : 406 rues OSM, 4 murs (troisième enceinte 1302-1327 en front de terre + mur de Garonne, castrum, deuxième enceinte), 31 portes, 13 monuments, 5 quartiers ; `docs/wip/rs-g-bordeaux.md` | fait, relu (20 / 10 / 4) |
| 1b | Avignon : 486 rues, enceinte du XIIIe s. ouverte + remparts 1357-1373, 13 portes, pont Saint-Bénézet (22 arches), 30 monuments datés, 4 quartiers ; `docs/wip/rs-g-avignon.md` | fait, relu (20 / 8 / 10) |
| 1c | Calais : 55 rues, enceinte de Hurepel (1228) + double fossé 1346, 8 portes, 13 monuments, havre `hand` ; `docs/wip/rs-g-calais.md` | fait, relu (13 / 2 / 13) |
| 1d | Bruges : 778 rues, levée de 1297 démantelée en 1328, 19 états de portes datés, 28 monuments, reien OSM dessinées ; `docs/wip/rs-g-bruges.md` | fait, relu (20 / 3 / 4) |
| 1e | Relectures `docs/histoire/relecture-vh-{bordeaux,avignon,calais,bruges}.md` (confirmés / corrigés / incertains) | fait |
| 2 | Paris : monuments fusionnés par cellule de 600 m (`TownBuilder._extra_meshes`, `merge_monuments`, shader `base_source` 2 : base, ancrage et teinte par sommet dans `CUSTOM0`) | fait, testé (plan) ; i/s non remesurés (pas de GPU en headless) |
| 3a | Bande sèche le long des rives : `river_widths.json` Rouen 200 → 260 m, Orléans 350 → 440 m, Tours 400 → 460 m (+ ancrages voisins bornant l'effet) | données faites ; recuisson par le joueur |
| 3b | Couloir de fleuve `towns_1340.json` : `towns.pick_river_feature` (un fleuve ≥ 4 × plus large que le cours d'eau le plus proche l'emporte) ; Tours 15 → 400 m, 19 autres villes corrigées (Nantes, Nevers, Bayonne, Cosne…) ; Orléans avait déjà la Loire (350 m) | fait, `geo towns` relancé |
| 3c | Bosses GLO-30 de la Cité de Londres : E3/E4 gardaient le modèle de surface (immeubles) sous les zones de détail ; `detail_dem.apply_parent_updates` cascade les moyennes 2 × 2 jusqu'à E3, `BAKE_VERSION` 6 | code fait et testé ; recuisson par le joueur |
| 4 | pytest 874 OK (1 échec hors lot : `test_budget`, ligne « F0 » de `docs/budget.md` venue de `main`), `vh4_landmarks_test` (y compris `_test_vh8` des 4 villes), `zg6_towns_test`, `smoke` OK après fusion de `main` | fait |

## Mesures (scripts de travail, non versionnés)
- Chenal du relief (cellules ≤ eau + 0,6 m le long de la normale) contre demi-largeur du ruban :
  Orléans (E7) p90 jusqu'à 260-324 m d'un côté, 30-180 m de l'autre, ruban 175 m ; Tours (E4) p90
  jusqu'à 224-333 m, ruban 200 m ; Rouen île Lacroix (E7) p90 130-193 m, ruban 100 m.
- Cité de Londres (résidu à la médiane 120 m) : E3 max 24,7 m, E4 max 51,9 m (GLO-30, immeubles),
  E5-E7 max 4-8 m (LiDAR EA).
- Plans headless (relief plat) : Bordeaux 4 328 maisons, Avignon 4 167, Calais 1 143, Bruges 8 706.

## Commandes à lancer par le joueur (après fusion dans `main`, depuis le dépôt principal)
```sh
uv run --project tools cent-ans geo relief-all --check   # tier3 signalé périmé (BAKE_VERSION 6)
uv run --project tools cent-ans geo relief-all           # recuit E5-E7 + cascade E4/E3, puis hydro-fine (largeurs) et anchors-fine (≈ 30-45 min)
uv run --project tools cent-ans geo towns                # couloirs de fleuve sur les nouveaux fleuves fins (≈ 5 min)
uv run --project tools cent-ans geo landmarks            # eaux `rivers_fine` des villes v2 (Rouen, Orléans, Bordeaux, Avignon)
uv run --project tools pytest -q tools/tests/test_landmarks_v2*.py tools/tests/test_towns.py
godot --headless --path game --script res://tests/vh4_landmarks_test.gd
```
Puis vérifier que `geo landmarks` ne pose pas de couloir trop large à Orléans et Rouen (ruban
élargi : `vh4_landmarks_test` doit rester OK) ; paquet de relief (`relief-pack`) à refaire avant
diffusion.

## Limites / suites
- Rendu jamais regardé en jeu (aucune capture dans ce lot) : Bordeaux (deux murs à 20-30 m à
  l'ouest, historiques), Avignon (pont coudé en deux entrées), Calais (havre non dessiné par le
  moteur), Bruges (levée rendue en mur bas).
- Manques de format : `certainty`/`note` sur les portes, levée de terre linéaire, pont coudé, eaux
  datées, tour de `church` à l'est, `belfry` sans flèche ni octogone ; polygones d'eau non dessinés.
- Sources non lues (listées dans chaque relecture) : Atlas historique de Bordeaux (2009), Rolland
  (1989) et Maynègre (1991) pour Avignon, Héliot (1947), Lenoir (2001) et *King's Works* pour
  Calais, Ryckaert (1991) pour Bruges.
- La largeur des fleuves n'efface pas le décalage de l'axe fin dans le chenal (Orléans : axe
  contre une rive) : un recalage de l'axe (`valley_snap`) serait la vraie correction.

## Journal
- 28/09 : squelette ; quatre agents villes, deux agents historiens ; lots 2, 3a-3c ; fusion de
  `main`, tests.
