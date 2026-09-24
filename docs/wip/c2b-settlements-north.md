# C2b — colonies des îles Britanniques, Pays-Bas, Empire, Scandinavie

Tâche : `docs/design/2026-09-24-echelle-colonies.md` § 3.1. 56 provinces (régions
`angleterre_*`, `galles`, `ecosse`, `irlande`, `pays_bas`, `empire_*`, `scandinavie`).

## État

En cours. Fait : tout `empire_*`, `scandinavie`, `pays_bas`, `angleterre_sud`, `galles`,
`angleterre_centre`, `angleterre_est`, `angleterre_nord`, et `ecosse` : lothian, fife —
50/56 provinces, 226... (compte final à la fin). Enclaves posées : Stirling (`fac_england`,
prov_fife), Dunbar (`fac_scotland`, prov_lothian).

## Prochaine étape

Continuer `ecosse` (galloway, highlands, renfrew) puis `irlande` (dublin, munster,
ulster) — dernier lot. Valider avec le script de validation temporaire (schéma
`settlement.schema.json` + `common.schema.json` via `referencing`), commit `wip:`
à la fin.
