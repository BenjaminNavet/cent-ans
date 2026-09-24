# C2b — colonies des îles Britanniques, Pays-Bas, Empire, Scandinavie

Tâche : `docs/design/2026-09-24-echelle-colonies.md` § 3.1. 56 provinces (régions
`angleterre_*`, `galles`, `ecosse`, `irlande`, `pays_bas`, `empire_*`, `scandinavie`).

## État

En cours. Fait : tout `empire_*`, `scandinavie`, tout `pays_bas` (10 provinces), et
`angleterre_sud` : cornwall, devon, kent — 36/56 provinces, 147 colonies.

## Prochaine étape

Continuer `angleterre_sud` (middlesex, sussex, wessex) + `galles` (deheubarth, gwynedd) ;
puis `angleterre_centre/est/nord` + `ecosse` + `irlande`. Valider avec le script de
validation temporaire (schéma `settlement.schema.json` + `common.schema.json` via
`referencing`), commit `wip:` toutes les ~8 provinces.
