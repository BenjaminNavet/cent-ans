# VH5 — Paris vers 1340 à l'échelle 1:1 (format landmark v2, ADR 0078)

Branche `feat/vh5-paris` (worktree d'agent, depuis `main` 89bc960a). Orchestration SZ
(`docs/wip/sz-suites-zoom.md`), chantier VH (`docs/wip/vh-villes-historiques.md`). Référence :
`docs/landmarks-v2.md`. Liens symboliques non versionnés `data/map/pyramid`, `tools/geo/raw` →
dépôt principal ; dylib copiée de `game/bin/` du dépôt principal. Aucun changement Rust.

## État
- [x] Squelette (ce fichier)
- [ ] Parcellaire ALPAGE : disponibilité vérifiée
- [ ] `data/landmarks_v2/paris.json` : origine, enceintes, portes, îles, ponts, monuments, quartiers, places
- [ ] Recette OSM (`exclude` haussmannien) + `geo landmarks`
- [ ] Rues disparues tracées à la main
- [ ] Rendu en jeu, captures `docs/img/vh5/`, i/s
- [ ] Tests (vh4_landmarks_test étendu, zg4, zg6, smoke, pytest)

## Prochaine étape
Vérifier ALPAGE, puis écrire paris.json.
