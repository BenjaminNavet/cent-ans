# TS-T3b — brume de vallée relative (suite de T3, ADR 0710)

Branche `ts/t3b`, worktree `../gp-ts-t3b`. Défaut : bassin parisien (2.35,48.85,200, printemps clair) noyé dans un voile laiteux.

## Correctif
- `post.wesl` : la brume de vallée ne suit plus l'altitude absolue mais les creux RELATIFS : hauteur du pixel comparée à la moyenne d'un anneau de 16 voisins écran (profondeur → y monde) ; plaine plate ou pente régulière = 0, fond de vallée > 0.
- La brume de plaine (bande d'altitude absolue) n'est plus qu'une part `plain_*` : 0 par temps clair, un peu le matin, forte en brouillard.
- Plafond par temps clair `clear_max`, brume du matin allégée, `near_share` 0,15 (sol net près du foyer).

## État
- [x] code + données + schéma, tests unitaires
- [x] captures paris / alps_mid / plaine d'hiver (Płock, temps clair forcé) ; fmt, clippy, 1348 tests, schéma, smoke 10/10

## Prochaine étape
Terminé, en attente de fusion.

## Points ouverts
- La vue Paris n'est pas en temps clair : la météo réelle au foyer est « brouillard » (intensité 0,92). Le voile blanc lointain restant vient surtout du brouillard de distance de l'ADR 0596 (`weather/atmosphere.rs`, hors périmètre de ce lot) ; la perspective aérienne de T3 est réduite de 60 % sous brouillard (`aerial.fog_cut`) pour ne pas doubler.
- Brume de vallée : 16 lectures de profondeur de plus par pixel (anneau de voisins).
