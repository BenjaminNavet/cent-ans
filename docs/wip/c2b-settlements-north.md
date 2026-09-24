# C2b — colonies des îles Britanniques, Pays-Bas, Empire, Scandinavie

Tâche : `docs/design/2026-09-24-echelle-colonies.md` § 3.1. 56 provinces (régions
`angleterre_*`, `galles`, `ecosse`, `irlande`, `pays_bas`, `empire_*`, `scandinavie`).

## État

En cours. Fait : tout `empire_*`, `scandinavie`, tout `pays_bas`, tout `angleterre_sud`,
tout `galles`, et `angleterre_centre` : gloucester, oxford, warwick — 44/56 provinces,
185 colonies.

## Prochaine étape

Continuer `angleterre_est` (norfolk), `angleterre_nord` (lancashire, northumberland,
yorkshire), puis `ecosse` (5) et `irlande` (3). Valider avec le script de validation
temporaire (schéma `settlement.schema.json` + `common.schema.json` via `referencing`),
commit `wip:` toutes les ~8 provinces.
