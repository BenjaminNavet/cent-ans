# bm-campveg5 : végétation et campagne (décor) — forêt, ombres, côtes

Branche `bm/campveg5` (base 42068a22b). Dossiers : `client/src/campaign/vegetation`, `client/src/campaign/countryside`.

## État
- Compile, clippy propre (commit wip). Tests en cours.
- FAIT (à vérifier en vidage) : vraies ombres portées des arbres (`vegetation/shadow_pass.rs` + `shadow_cast.wesl`), bascule par distance
  (`vegetation/shadows.rs`, 70 + hystérésis 8 %, plafonné par `veg_shadow_distance` du préréglage) ; ellipses retirées.
- FAIT (à vérifier) : panaches de fumée du décor (`countryside/decor_smoke.rs`, flipbook fumée du joueur).
- FAIT (à vérifier) : rochers de côte (`countryside/coast_rocks.rs`, `coast_dist.png` + `coast_types.json`).

## Prochaine étape
- Compiler, tests, vidages d=25/60/200 Paris été/hiver.
