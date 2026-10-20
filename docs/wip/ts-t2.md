# TS-T2 — forêts style Total War (ADR 0710)

Branche `ts/t2`, worktree `/Users/jean_hubert/dev/gp-ts-t2`. Plan : `docs/wip/ts-style-total-war.md`.

## État
- Squelette : `data/art/campaign_forest_look.json` + schéma, `vegetation/look.rs` (lecture).

## Diagnostic
- Atlas d'imposteurs : couleur moyenne opaque sRGB (82, 80, 58), kaki terne → canopée olive/beige.
- Tramage visible : Bayer 4×4 entre deux vues d'azimut et dans la bande modèle/imposteur.

## Prochaine étape
- Brancher `ForestLook` dans les paramètres des lots (shader), lisière dans `edges.rs`.
