# FR1 — Frontières de faction lumineuses (façon Total War)

Branche : `feat/fr1-faction-borders`. ADR : `docs/decisions/0070-frontieres-de-faction.md`.

## Approche (retenue après mesure)

Crochet dans le fragment du terrain : `game/shaders/faction_borders.gdshaderinc`
(`fr1_borders(p, uv, footprint, albedo, emission)`), inclus par `terrain.gdshader` et
`terrain_parchment.gdshader` (2 lignes chacun : include + appel, marquées « FR1 »). Uniformes
`fr1_*` posés sur le matériau partagé du terrain par `game/scripts/map/faction_borders.gd`
(`FactionBorders`), branché dans `campaign_map.gd` (setup, `refresh_all`, `_process`).
Réglages : `data/map/faction_borders.json` (+ schéma, pytest `tools/tests/test_faction_borders_schema.py`).

Premier essai abandonné : passe `next_pass` du matériau du terrain (shader séparé qui rejoue le
`qt_vertex` du quadtree) — drapé parfait mais +1,1 ms (Europe) à +4,1 ms (comté) de GPU : tout le
relief est redessiné (sommets du quadtree + tous les pixels). Le crochet ne coûte que les pixels
proches d'une frontière.

## État

- [x] Include + contrôleur + données/schéma + test headless `fr1_borders_test.gd` (OK).
- [x] Captures fenêtrées `docs/audit/captures/fr1/` (Europe, comté, vallée, parchemin, avec/sans).
- [ ] Mesure A/B entrelacée (Vulkan) après bilinéaire précis de près.
- [ ] ADR 0070.
- [ ] Vérifs finales : cargo fmt/clippy/test, pytest, build, import, smoke, `git merge main`.

## Prochaine étape

Relancer `fr1_shot.gd` (vulkan), regarder la vue vallée (pointillés corrigés ?), écrire l'ADR.
