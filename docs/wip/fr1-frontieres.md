# FR1 — Frontières de faction lumineuses (façon Total War)

Branche : `feat/fr1-faction-borders`. ADR : `docs/decisions/0070-frontieres-de-faction.md`.

## Approche

Passe suivante (`next_pass`) du matériau partagé du terrain : `game/shaders/faction_borders.gdshader`
reprend exactement le chemin de sommets du terrain (`qt_vertex` ZG2, relief exagéré ZG8, lit
creusé V4 ; plat en vue parchemin seule), donc drapée sur le relief à tous les zooms, y compris
sur les patchs du quadtree. **terrain.gdshader n'est pas modifié** (aucun crochet ajouté).
Contrôleur : `game/scripts/map/faction_borders.gd` (`FactionBorders`), branché dans
`campaign_map.gd` (setup, `refresh_all`, `_process`). Réglages : `data/map/faction_borders.json`
(+ schéma, pytest `tools/tests/test_faction_borders_schema.py`).

## État

- [x] Squelette + shader + contrôleur + données/schéma + test headless `fr1_borders_test.gd` (OK).
- [ ] Captures fenêtrées (3 zooms + parchemin) dans `docs/audit/captures/fr1/`.
- [ ] Mesures de perf (3 zooms, High, 1080p, A/B `--no-faction-borders`).
- [ ] ADR 0070.
- [ ] Vérifs finales : cargo fmt/clippy/test, pytest, build, import, smoke, `git merge main`.

## Prochaine étape

Lancer le jeu fenêtré, regarder les frontières, régler les largeurs, mesurer.
