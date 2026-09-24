# C2b — colonies des îles Britanniques, Pays-Bas, Empire, Scandinavie

Tâche : `docs/design/2026-09-24-echelle-colonies.md` § 3.1. 56 provinces (régions
`angleterre_*`, `galles`, `ecosse`, `irlande`, `pays_bas`, `empire_*`, `scandinavie`).

## État

En cours. Fait : `empire_rhin` (alsace, cologne, lorraine, mainz, palatinate, trier),
`empire_est` (austria, bohemia, brandenburg, meissen) — 10/56 provinces, 40 colonies.

## Prochaine étape

Continuer : `empire_sud` (bern, franconia, oberbayern, swabia, tirol, waldstatten),
puis `empire_nord` + `scandinavie` + début `pays_bas`. Valider avec le script de
validation temporaire (schéma `settlement.schema.json` + `common.schema.json` via
`referencing`), commit `wip:` toutes les ~8 provinces.
