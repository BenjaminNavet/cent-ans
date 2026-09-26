# SZ2b — nappe d'eau des fleuves au palier site (suite de SZ2)

Chantier SZ (`docs/wip/sz-suites-zoom.md`). Branche `sz2b-water-sheet` (worktree d'agent
`agent-a098d00691a342944`, depuis `main` 89bc960a). Liens symboliques non versionnés
`data/map/pyramid`, `tools/geo/raw` → dépôt principal ; dylib copiée du dépôt principal.
Ne touche ni aux villes (VH5-VH7), ni à l'exagération (SZ1).

## Défaut
Au palier site, la Seine à Rouen s'affiche en lit sableux gris sans nappe d'eau, grève verte entre
mur de rive et lit (`docs/img/vh4/rouen_site.jpg`, `rouen_pont.jpg`, `rouen_seine_sud.jpg`).

## Plan
1. Diagnostic : niveau d'eau `hydro_fine` vs lit creusé vs relief, ruban construit / masqué,
   Rouen puis Loire (Orléans, Tours), Tamise (Londres), Garonne (Bordeaux), 3 paliers.
2. Correctif à la source (outils et/ou rendu).
3. Captures avant/après `docs/img/sz2b/`, tests zg5b_fine_geo, zg7a, smoke, pytest.

## État
- [ ] 1. diagnostic
- [ ] 2. correctif
- [ ] 3. captures, tests, docs

## Prochaine étape
Diagnostic.
