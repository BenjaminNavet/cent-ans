# TS-T2b — disques noirs sous les arbres, fond de vallée noir

Branche `ts/t2b`, worktree `/Users/jean_hubert/dev/gp-ts-t2b`. Suite de `docs/wip/ts-t2.md`.

## État
- Disques noirs : socle plat au pied de modèles générés (épicéa : 11 % des triangles, disque large
  comme le houppier). Sur pente, il dépasse en aval, normales radiales vers le bas → noir.
  Correctif : `render.rs::foot_plate_free` retire les triangles bas (< 8 % de la hauteur) hors du tronc.
- Fond de vallée : diagnostic en cours (mesures de luminance de captures avec `CENT_ANS_PERF_OFF`).

## Prochaine étape
- Mesurer la part post / végétation / ombres / sol forestier, corriger dans les données.
