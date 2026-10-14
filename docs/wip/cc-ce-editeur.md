# CC CE — éditeur de carte local

Branche `cc/ce` (worktree `game_project-cce`). Contrat : `docs/wip/cc-carte-composee.md`. Doc : `docs/editeur-carte.md`.

## État : TERMINÉ (tests 19/19, page non vue dans un navigateur : syntaxe JS vérifiée par `node --check` seulement)
- `compose/nudges.py` + `place_nudges.json` + schéma ; `relief.load_inputs` l'applique.
- Crêtes `authored` = source : `ridges_author.apply` (amorçage unique `chains_seeded`), `compile.keep_authored_ridges`, `deleted_ids`.
- `compose/editor.py` (stdlib) + `editor_static/index.html` ; `cent-ans compose editor`.
- Aperçu « Planche » : relief de la fenêtre visible, 2-5 s.

## Points ouverts
- Œil du joueur sur la page (pas de navigateur lancé par l'agent).
- `geo compose` complet non relancé après les changements (test unitaire de `keep_authored_ridges` seulement).
- C4 : consommer `nudges.place_positions` pour toute position composée de lieu ; masques `masks/*.png` à lire.
