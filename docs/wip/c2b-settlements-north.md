# C2b — colonies des îles Britanniques, Pays-Bas, Empire, Scandinavie

Tâche : `docs/design/2026-09-24-echelle-colonies.md` § 3.1. 56 provinces (régions
`angleterre_*`, `galles`, `ecosse`, `irlande`, `pays_bas`, `empire_*`, `scandinavie`).

## État

En cours. Fait : `empire_rhin`, `empire_est`, `empire_sud` (bern, franconia, oberbayern,
swabia, tirol, waldstatten), `empire_nord` (frisia, holstein, lower_saxony, westphalia)
— 20/56 provinces, 77 colonies.

## Prochaine étape

Continuer : `scandinavie` (gotaland, jutland, sjaelland) puis `pays_bas` (10 provinces).
Valider avec le script de validation temporaire (schéma `settlement.schema.json` +
`common.schema.json` via `referencing`), commit `wip:` toutes les ~8 provinces.
