# M1 — grille de navigation (état)

Spec : `docs/design/2026-09-24-mouvement-libre.md` § 2. Branche : `worktree-agent-aded02b061ab29ed8`.

## État

- [x] Squelette : `data/movement/rules.json` + schéma, `data/schemas/crossings.schema.json`, `tools/cent_ans_tools/geo/navgrid.py` (API vide).
- [ ] `data/map/crossings.json` (ponts, gués, cols sourcés).
- [ ] Rastérisation : terrain, pente, mer, routes, fleuves, passages, cols, colonies.
- [ ] Contrôles (connexité, îles sans port), aperçu `docs/img/navgrid-preview.png`, `map.json.navgrid`.
- [ ] Tests pytest, CLI `cent-ans geo navgrid`, branchement dans `geo/build.py`.

## Prochaine étape

Écrire `crossings.json`, puis l'implémentation de `navgrid.py`.
