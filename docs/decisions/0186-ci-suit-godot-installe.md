# 0186 — La CI suit la version de Godot installée

Date : 2026-10-08

## Contexte

Homebrew met Godot à jour chaque nuit sur la machine de développement (`brew upgrade --greedy-auto-updates`, tâche `com.homebrew.autoupdate`). La CI Windows épingle une version fixe (`GODOT_VERSION` dans `.github/workflows/windows.yml`). Sans alignement, le jeu serait développé sur une version et testé sur une autre, et une nouvelle série (4.8) réimporterait le projet sans décision.

## Décision

La version installée sur la machine de développement fait foi ; la CI la suit. `tools/sync_godot_version.sh` lit `godot --version` et, si elle diffère de `GODOT_VERSION`, réécrit le workflow et les mentions de version du `README.md` ; si la série mineure change, aussi `GODOT_SERIES` (`tools/launch.sh`) et `config/features` (`game/project.godot`). Il commite ces seuls chemins, ou s'abstient s'ils ont d'autres modifications locales.

`tools/launch.sh` l'appelle à chaque lancement sur la branche `main` (jamais en CI, jamais sur `stable` ni sur une branche de travail). Seules les versions `stable` sont épinglées.

## Conséquences

- Le commit d'alignement part avec le prochain `git push` ; la CI teste alors la nouvelle version et ne promeut `stable` que si le test de fumée passe (ADR 0159).
- Un changement de série impose aux joueurs Windows d'installer la nouvelle version de Godot ; le lanceur les prévient (« le projet attend Godot x.y »).
- L'alignement n'a lieu qu'au lancement du jeu depuis `main`, pas au moment exact de la mise à jour Homebrew.
