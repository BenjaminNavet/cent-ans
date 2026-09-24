# M1 — grille de navigation (état)

Spec : `docs/design/2026-09-24-mouvement-libre.md` § 2. Branche : `worktree-agent-aded02b061ab29ed8` (non fusionnée).

## État : terminé

- `tools/cent_ans_tools/geo/navgrid.py` : étape du pipeline, appelée à la fin de `geo/build.py` et seule par `uv run --project tools cent-ans geo navgrid` (`--lenient` écrit la grille malgré des colonies isolées).
- `data/map/navgrid.png` : 2048², 8 bits, 0,37 Mo ; `map.json` gagne `navgrid`.
- `data/map/crossings.json` + `data/schemas/crossings.schema.json` : 79 ponts, 17 gués ou bacs (typés `ford`), 12 cols avec leur route.
- `data/movement/rules.json` + `data/schemas/movement_rules.schema.json` : clés exactes de la spec, partagées avec M2.
- Aperçu : `docs/img/navgrid-preview.png` (rampe de coût, routes orange, grands fleuves bleus, passages rouges, cols jaunes, colonies blanches).
- Tests : `tools/tests/test_navgrid.py` (18 tests : unitaires synthétiques, schémas, fichier à jour, connexité, fleuves, ponts, cols).

## Chiffres

- 97,9 % des cases de terre franchissables.
- Passages : 91 ponts et gués appliqués, 397 croisements route Itiner-e / grand fleuve, 12 cols ; 27 des 569 colonies sont sur un grand fleuve et l'ouvrent (case de colonie franchissable, règle de la spec).
- Seuil de pente infranchissable : 0,22 (dénivelé / distance sur une case de 1,44 km). À 0,20, Altdorf (Uri) était isolée ; la route du Saint-Gothard part désormais d'Altdorf.
- Aucune colonie isolée. Îles sans port signalées : `set_pomposa` (delta du Pô), `set_teylingen` (Hollande) ; toutes deux sont sur des îlots du masque d'eau (terres sous 0 m dans ETOPO).

## Règles de construction

- Terrain : classe la plus chère parmi plaine (10), collines (≥ 350 m ou pente ≥ 0,05 : 15), forêt (poids forêt du splat ≥ 0,5 : 18), marais (province `marsh`, ≤ 15 m, plat : 25), montagne (≥ 1 200 m, pente ≥ 0,14 ou roche ≥ 0,45 : 30).
- Cours d'eau secondaires +10, routes ×0,75 (1 case), puis grands fleuves à 255.
- Les grands fleuves sont rastérisés `all_touched` puis fermés en diagonale : un pas en 8 voisins ne peut pas passer entre deux cases de fleuve qui se touchent par un coin. Le cœur (M2) peut donc autoriser les diagonales sans règle de coin.
- Pont ou gué : ouvre les cases du fleuve nommé dans un rayon de 1 case autour de la case de fleuve la plus proche (recherche à 8 cases).
- Croisement route Itiner-e / grand fleuve : chevauchements de 6 cases au plus (au-delà, la route longe le fleuve et ne l'ouvre pas).
- Col : chemin de moindre coût entre les points de sa route (pentes autorisées, eau et fleuves interdits), forcé au coût montagne.

## Limites

- `rivers.geojson` (Natural Earth) ne trace pas la Saône en aval de Chalon : les ponts de Lyon (Saône), Mâcon, Tournus et Chalon n'ont pas d'effet (listés « hors du tracé »). Le Rhin s'arrête au delta : Waal, Lek, Nederrijn et IJssel restent des cours d'eau secondaires.
- La Somme n'est pas un grand fleuve : Blanchetaque est documenté mais sans effet.
- Les 397 croisements route/fleuve viennent des voies romaines : ils ouvrent plus de passages que les ponts de 1337 (fidèle à la spec, à revoir à l'équilibrage M5 si les fleuves ne pèsent pas assez).
- Les bacs sont typés `ford` (le schéma n'a que pont, gué, col).
- `splat.png` n'est pas régénéré par `geo build` ; la grille lit la version présente.

## Prochaine étape

Fusion dans main puis M2 (lecture de `navgrid.png` et de `data/movement/rules.json` dans `GameData`).
