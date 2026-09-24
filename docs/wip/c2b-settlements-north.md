# C2b — colonies des îles Britanniques, Pays-Bas, Empire, Scandinavie

Tâche : `docs/design/2026-09-24-echelle-colonies.md` § 3.1. 56 provinces (régions
`angleterre_*`, `galles`, `ecosse`, `irlande`, `pays_bas`, `empire_*`, `scandinavie`).

## État

En cours. Fait : tout `empire_*`, `scandinavie`, et `pays_bas` : brabant, flandre,
flandre_wallonne, gueldre, hainaut — 28/56 provinces, 111 colonies.

## Prochaine étape

Continuer `pays_bas` : holland, liege, luxembourg, namur, utrecht ; puis
`angleterre_sud` + `galles`. Valider avec le script de validation temporaire (schéma
`settlement.schema.json` + `common.schema.json` via `referencing`), commit `wip:`
toutes les ~8 provinces.
