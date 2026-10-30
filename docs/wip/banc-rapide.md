# Banc rapide (outillage de vérification)

## État
- Périmètre réduit par le joueur : seul `tools/checks_for_diff.sh <base>` est livré. Abandonnés, non faits : `full_check.sh` en deux chaînes parallèles sous nextest, et `eco_series` multi-graines via `ai::parallel`.
- `tools/checks_for_diff.sh` lit `git diff --name-only <base>...HEAD`, imprime `RUN <nom> <commande> # fichiers déclencheurs` puis `SKIP <noms>` ; il ne lance rien.
- Règles : battle_bench (sim-battle, ai, data/rules/battle_*, data/unit_types), fixture auto-resolve (unit_types, battle_*), eco_series (sim-campaign, ai, game-api, règles et données de campagne), seed bots --quick (core/, data/), smoke (client/, game/), pytest (tools/), full_check (tout sauf docs seuls).
- Règle 7 de `docs/prompt-session.md` mise à jour (une phrase).

## Testé
- `HEAD~5` (outils seuls) : full_check + pytest. `HEAD~40` : tout sauf la fixture. `add64db30~1` (data/unit_types) : voir le rapport.

## Pistes
- Reprendre les points 1 et 2 si le joueur les relance (mesure avant/après à faire alors).
