# WIP F2 — Icônes et infobulles

Branche : `worktree-agent-af998097891c4cf5c`. Plan : `docs/design/v2-finalisation.md` (lot F2).

## État : terminé
- [x] Outil Python `tools/cent_ans_tools/icons.py` + `icons_catalog.py`, CLI `cent-ans assets icons`, tests `tools/tests/test_icons.py` (9).
- [x] 144 identifiants → 106 SVG dans `game/assets/icons/` + `icons.json` + `.import` (mipmaps).
- [x] Autoload `IconLibrary` (`game/scripts/ui/icon_library.gd`) déclaré dans `project.godot`.
- [x] `GameCatalog` (définitions `data/` pour l'affichage) + `RichTooltip`, `RichButton`, `RichPanel`, `RichLabel`, `IconChip` ; styles `TooltipPanel`/`TooltipLabel` dans `parchment_theme.tres`.
- [x] Intégration : barre du haut, province, armée, bataille, technologies, cour, fiche personnage, faction ; stage `--stage=tooltips`.
- [x] Smoke `_run_icons` ; smoke complet vert (13 étapes OK).
- [x] `CREDITS.md`, captures `docs/img/godot-icons-*.png`, `docs/status.md`, `docs/tools.md`.

## Prochaine étape
Fusion par l'orchestrateur. Suite possible (hors F2) : accesseurs Rust `get_unit_type`/`get_building`/`get_resource` dans `GameDataStore` pour remplacer `GameCatalog`.

## Notes
- Cache des SVG bruts : `~/.cache/cent-ans/game-icons` (partagé entre worktrees).
- `game/bin/*.dylib` copiés/reconstruits localement (non versionnés).
