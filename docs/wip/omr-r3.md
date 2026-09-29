# OMR R3 — équilibre Est (commise, banqueroutes, révoltes)

Worktree `../gp-omr-r3`, branche `feat/omr-r3`. Cible cargo partagée `core/target`, profil `r3`.

## Commande de mesure
`CARGO_TARGET_DIR=<main>/core/target cargo build -p ai --example century_probe --config 'profile.r3.inherits="release"' --config 'profile.r3.debug=0' --profile r3`
puis `<main>/core/target/r3/examples/century_probe <tours> <graines…>` (`VERBOSE=1` : banqueroutes par faction).

## Référence (om-i1, 50 t. × 5)
Banqueroutes 5,25 / fac. / déc. ; petites fac. 6,03-6,47 ; 28 anciennes 0,03-0,23 ; révoltes 15,2 / 200 t. [4-36] ;
commise de Guyenne 5/5 au tour 1.

## État
- [ ] Commise de Guyenne : diagnostic.
- [ ] Banqueroutes des petites factions.
- [ ] Révoltes.
- [ ] Sonde 464 t. × 10.

## Prochaine étape
Mesure de référence verbeuse (banqueroutes par faction).
