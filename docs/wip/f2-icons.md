# WIP F2 — Icônes et infobulles

Branche : `worktree-agent-af998097891c4cf5c`. Plan : `docs/design/v2-finalisation.md` (lot F2).

## État
- [x] Outil Python `tools/cent_ans_tools/icons.py` + `icons_catalog.py`, CLI `cent-ans assets icons`, tests `tools/tests/test_icons.py`.
- [x] 144 identifiants → 106 SVG dans `game/assets/icons/` + `icons.json`.
- [x] Autoload `IconLibrary` (`game/scripts/ui/icon_library.gd`) déclaré dans `project.godot`.
- [x] Catalogue de données d'affichage (`GameCatalog`) + infobulles riches (`RichTooltip`, `RichButton`, `RichPanel`, `RichLabel`, `IconChip`).
- [x] Intégration : barre du haut, province, armée, bataille, technologies, cour, fiche personnage, faction ; stage `--stage=tooltips`.
- [x] Smoke « icons » (vert, smoke complet vert).
- [ ] `CREDITS.md`, captures, `docs/status.md`, `docs/tools.md`.

## Prochaine étape
CREDITS.md, captures `docs/img/godot-icons-*.png`, docs status/tools.

## Notes
- Cache des SVG bruts : `~/.cache/cent-ans/game-icons` (partagé entre worktrees).
- `game/bin/*.dylib` copiés depuis le dépôt principal (non versionnés).
