# TS-T2 — forêts style Total War (ADR 0710)

Branche `ts/t2`, worktree `/Users/jean_hubert/dev/gp-ts-t2`. Plan : `docs/wip/ts-style-total-war.md`.

## État
- Squelette : `data/art/campaign_forest_look.json` + schéma, `vegetation/look.rs` (lecture).
- Branché (compile, tests verts, pas encore vu en jeu) : canopée (valeur, saturation, balance,
  variété arbre/peuplement, ombre du bas du houppier, variété de hauteur), vues d'azimut fondues
  (plus de Bayer), bande modèle/imposteur arbre par arbre, agrandissement lointain, givre des
  conifères / feuillus nus gris-brun, lisière nette + isolés/bosquets (`edges.rs`), limite des
  arbres franche + roche des pentes (`mask.rs`).

## Diagnostic
- Atlas d'imposteurs : couleur moyenne opaque sRGB (82, 80, 58), kaki terne → canopée olive/beige.
- Tramage visible : Bayer 4×4 entre deux vues d'azimut et dans la bande modèle/imposteur.

## Prochaine étape
- Lancer le client (shader compilé ?), dump `vegetation`, captures de diagnostic (veg coupée à
  forest_mid et town_winter pour attribuer taches beiges / blocs gris), réglages.
