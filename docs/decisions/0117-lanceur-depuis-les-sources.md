# 0117 — Lanceur depuis les sources (macOS, Linux, Windows)

Date : 2026-09-30. Statut : accepté.

## Contexte

Lancer le jeu depuis les sources demandait trois commandes à retenir : `core/build.sh` après
chaque changement du cœur Rust, `godot --headless --path game --import` après chaque changement
des ressources (clone, `git pull`, fusion), puis `godot --path game`. Oublier une étape donne
une bibliothèque périmée ou des ressources non importées. Le jeu lancé hors éditeur n'importe
rien lui-même. Chaque système avait sa propre procédure. Linux n'était pas déclaré dans
`cent_ans.gdextension`, et `core/build.sh` ne savait copier qu'une `.dylib`.

## Décision

- **Un script unique, `tools/launch.sh`** (bash, compatible avec le bash 3.2 de macOS et avec
  Git Bash sous Windows). Il fait trois choses dans l'ordre :
  1. **Cœur Rust** : il appelle toujours `core/build.sh`, et c'est cargo qui détecte les
     changements (une compilation sans changement prend environ 1 s). `build.sh` ne remplace
     la bibliothèque de `game/bin/` que si son contenu a changé (`cmp`). Si cargo est absent
     mais qu'une bibliothèque existe déjà, celle-ci est utilisée telle quelle.
  2. **Import headless** si l'une de ces conditions est vraie : pas de `.godot/` (premier
     lancement), empreinte différente, ou fichier plus récent que le dernier import.
     L'empreinte est un `cksum` de la version de Godot et de la liste des fichiers de `game/`,
     hors `.godot/` et bibliothèques compilées : elle détecte les ajouts et les suppressions.
     Le « fichier plus récent » détecte les modifications, y compris celles de `git pull`. Le
     tampon `game/.godot/cent_ans_import.stamp` est écrit après l'import, car l'import écrit
     lui-même des `.import`/`.uid`.
  3. **Lancement** : `godot --path game`. Les arguments placés après `--` sont transmis à
     Godot. Options : `--no-build` et `--import` (import forcé).
- **Recherche de Godot** : dans l'ordre, `$GODOT`, puis `godot`/`godot4` dans le PATH, puis les
  emplacements d'installation usuels de chaque système. Sous Windows, l'import passe par
  l'exécutable `_console.exe`, qui écrit dans le terminal. Le lanceur avertit si la version
  n'est pas 4.7.x.
- **Lanceurs à double-cliquer** à la racine : `Lancer Cent Ans.command` (macOS, Finder),
  `Lancer Cent Ans.sh` (Linux) et `Lancer Cent Ans.bat` (Windows). Le `.bat` retrouve le
  `bash.exe` de Git pour Windows (installation machine, installation utilisateur ou via
  `where git`) plutôt que le `bash` de WSL. Il garde la fenêtre ouverte en cas d'erreur.
  `.gitattributes` fixe les fins de ligne (LF pour `.command`, CRLF pour `.bat`).
- **Linux x86_64 et arm64 déclarés** dans `cent_ans.gdextension` (`libcent_ans.*.so`).
  `core/build.sh` choisit l'extension selon le système et délègue à `build-windows.sh` sous
  Git Bash.

## Conséquences

- Une seule procédure pour les trois systèmes : cloner, puis double-cliquer sur le lanceur.
- Linux fonctionne depuis les sources, sans export. Il n'y a pas encore de préréglage d'export
  ni de CI Linux.
- Un passage dans l'éditeur Godot touche des fichiers de `game/` : le lancement suivant refait
  un import, qui est rapide quand il n'y a rien à importer.
- La bibliothèque de `game/bin/` garde son horodatage quand rien n'a changé. Cela évite aussi
  d'écraser, sous Windows, une DLL chargée par un Godot ouvert.
