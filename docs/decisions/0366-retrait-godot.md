# ADR 0366 — Retrait du client Godot

Date : 17/10/2026. Statut : accepté.

## Contexte
Le client Bevy (`client/`, ADR 0357) a atteint la parité fonctionnelle ; `main` est la branche `bevy-migration` depuis le 17/10. Le client Godot (`game/scripts`, `game/scenes`, `game/shaders`, `game/tests`, GDExtension `godot-bridge`) n'est plus lancé ni testé et coûte du temps de CI, de build et de lecture.

## Décision
Retirer du dépôt tout ce qui ne sert qu'à Godot :
- `game/scripts`, `game/tests`, `game/shaders`, `game/scenes`, `game/tools`, `game/bin`, `game/project.godot`, `game/export_presets.cfg`, tous les `*.import`, `.gdignore` et `*.uid` ;
- le crate `core/crates/godot-bridge`, `core/build.sh`, `core/build-windows.sh` ;
- les scripts de bancs, captures, export et synchronisation Godot de `tools/`, le job `smoke` Godot de la CI, le serveur MCP « godot » ;
- l'écriture de fichiers `.import` par les outils Python.

Exceptions conservées :
- `game/assets/` : racine d'assets du client Bevy (`AssetPlugin`), sous son chemin historique pour ne pas déplacer ~10 000 fichiers.
- `game/resources/*.tres` : cinq sont lus par Bevy (`client/src/campaign/terrain/tres.rs`) ; les autres (forest_detail, map_prop_scale) restent par cohérence. Conversion en JSON sous `data/` : reportée.
- `game/icon.png`, `icon.ico`, `icon.icns` : icônes du lanceur et de l'empaquetage.
- Les docs historiques sont conservés avec une ligne d'en-tête « Historique ».

## Conséquences
- Un seul client ; `tools/launch.sh` ne connaît que Bevy ; la CI ne garde que le job `bevy` et les tests du lanceur.
- Les commentaires « port de game/scripts/… » dans `client/src` restent valables comme références via l'historique git (dernier commit avec Godot : 9395b327a).
- Les anciennes références Godot ne sont plus exécutables ; pour une comparaison, récupérer ce commit dans un worktree.
