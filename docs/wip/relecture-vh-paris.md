# Relecture historique de Paris vers 1340 (VH5)

Branche : worktree d'agent `worktree-agent-a8c2568b3bd7b7929` (depuis `main`). Rapport :
`docs/histoire/relecture-vh-paris.md`.

## Méthode
- Corrections dans `tools/geo/paris_v2_author.py`, puis
  `uv run --project tools python tools/geo/paris_v2_author.py` et
  `uv run --project tools cent-ans geo landmarks --city paris` (reproductible : diff nul avant
  corrections).
- Liens symboliques non versionnés `data/map/pyramid`, `tools/geo/raw` ; dylibs copiées dans
  `game/bin/`.

## État
- [x] Recherches (ponts, Notre-Dame, enceintes, portes, Louvre, Temple, Saint-Victor, Bernardins)
- [ ] Corrections script + JSON
- [ ] Rapport
- [ ] Tests pytest + Godot vh4_landmarks_test
