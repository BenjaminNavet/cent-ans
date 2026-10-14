# CC CE — éditeur de carte local

Branche `cc/ce` (worktree `game_project-cce`). Contrat : `docs/wip/cc-carte-composee.md`.

## Plan
1. `compose/nudges.py` + `place_nudges.json` + schéma ; relief l'applique. (fait)
2. `ridges_author.apply` : crêtes `authored` = source (plus d'écrasement) ; `compile_all` les reporte. (fait)
3. `compose/editor.py` (serveur stdlib, API JSON) + `compose/editor_static/index.html` ; commande `cent-ans compose editor`.
4. Tests `tools/tests/test_compose_editor.py` ; doc `docs/editeur-carte.md`.

## État
- 1 et 2 faits, à tester ; 3 en cours.
