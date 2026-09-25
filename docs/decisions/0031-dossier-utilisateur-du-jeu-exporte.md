# 0031 — Dossier utilisateur propre au jeu exporté

Date : 2026-09-25 (lot PF1, performances et finitions de la release)

## Contexte
Le jeu exporté (`export/Cent Ans.app`) et le projet lancé depuis l'éditeur ou par les tests
(`godot --path game …`) utilisaient le même `user://` :
`~/Library/Application Support/Godot/app_userdata/Cent Ans`. Les sauvegardes automatiques, les
réglages et le codex du joueur s'y mêlaient à ceux des parties lancées par les agents et par les
tests (`smoke.json`, `settings_smoke.cfg`, `codex_test.json`, dossiers `rl1_journey_*`), et un
test pouvait écraser `auto_1.json` du joueur (point ouvert de RL1).

## Décision
- **Le jeu exporté a son propre dossier** : `~/Library/Application Support/Cent Ans`.
  `game/project.godot` porte `application/config/use_custom_user_dir.template=true` et
  `custom_user_dir_name.template="Cent Ans"` : réglages propres à la fonctionnalité `template`,
  présente seulement dans les modèles d'export. L'éditeur et les tests (binaire éditeur, sans
  `template`) gardent l'ancien dossier : aucun test ni outil d'agent n'est modifié.
- **Rien n'est perdu pour le joueur** : au premier lancement du jeu exporté, `UserDirMigration`
  (`game/scripts/ui/user_dir_migration.gd`, appelé par le premier autoload `MapPaths`, avant toute
  lecture de `user://`) **copie** une fois depuis l'ancien dossier `settings.cfg`, `codex.json` et
  les fichiers de `saves/` (sauvegardes, métadonnées, vignettes), sauf les fichiers de test
  (`smoke*`, `*_test*`, `*journey*`, `*_smoke.cfg`). Un fichier déjà présent n'est jamais écrasé ;
  l'ancien dossier n'est ni déplacé ni effacé ; le marqueur `.migrated_from_shared_userdata`
  empêche une seconde copie.
- Le cache Metal de macOS (préchauffé par `tools/export_macos.sh`) dépend de l'identifiant de
  l'application, pas de `user://` : inchangé.

## Alternatives écartées
- **Isoler l'éditeur et les tests** (`use_custom_user_dir.editor`) plutôt que l'export : les
  sauvegardes faites en jouant depuis l'éditeur (recettes Q1/Q2) auraient disparu de l'éditeur, et
  tous les chemins de test documentés auraient changé.
- **Déplacer** les fichiers au lieu de les copier : une erreur pendant le déplacement pouvait
  perdre une sauvegarde ; la copie garde toujours l'original.
- **Nom d'application distinct** (`config/name`) pour l'export : change aussi le titre de la
  fenêtre et l'identifiant du cache, pour le même effet.

## Conséquences
- Une partie sauvegardée dans le jeu exporté n'apparaît plus dans l'éditeur (et inversement après
  la première copie). Pour reprendre dans l'éditeur une sauvegarde du jeu exporté, la copier de
  `~/Library/Application Support/Cent Ans/saves` vers l'ancien dossier.
- Les journaux du jeu exporté (`logs/`) sont eux aussi dans le nouveau dossier.
- Autres plateformes : même mécanisme (`%APPDATA%\Cent Ans` sous Windows,
  `~/.local/share/Cent Ans` sous Linux) ; l'ancien dossier est calculé par
  `UserDirMigration.legacy_dir()` à partir de `OS.get_data_dir()`.
