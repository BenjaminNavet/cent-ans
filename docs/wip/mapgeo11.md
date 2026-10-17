# mapgeo11 — défauts géographiques et d'affichage du testeur visuel

Origine : `docs/wip/bm-testeur-visuel.md` (17/10). Branche `bm/mapgeo11`, pas de fusion.

## État
- Fait (code) : décalage libellés/maquettes (cause : `UiScale` non appliqué aux nœuds UI posés
  depuis `world_to_viewport`), parchemin (halo des frontières), Vilnius (famille de maquettes
  païenne, villes païennes en bois), côtes d'auteur (`inlets` / `headlands` de `generalise.json`).
- Fait (données, reprise 17/10) : `compose coast-splice` + `compose coast-patch` (relief E0-E3 et heightmaps, idempotent) puis splat, navgrid, colormap, sea-lanes, authored-px, decor_bake. Horizon NON régénéré (dérive sans rapport, ~213 fichiers). Provinces/routes/rivières inchangées (la retouche ne touche que 4 fenêtres d'estuaires).
- À faire : captures avant/après, tests, `.bm-ready`.

## Chaîne de régénération (côtes -> tout le reste), ordre de `docs/geo.md`
geo compose, compose-relief, compose-layers, provinces, settlements, roads --computed, splat,
rivers-render, hamlets, navgrid, colormap, sea-lanes, compose authored-px, horizon, cross_fixture,
puis `python -m cent_ans_tools.decor_bake` (px du Mont-Saint-Michel).

## Prochaine étape
Vérifier les diffs de données, captures Guyenne / Normandie / baie du Mont, tests, commit, `.bm-ready`.
