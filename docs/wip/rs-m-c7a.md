# RS-M : sonde m3 c7a rouge (trésor de la France)

Branche `feat/rs-m-c7a` (depuis main 427b4978). Cible cargo privée `core/target-rs-m` (à supprimer en fin de lot).

## Problème
`fifty_turns_on_eight_seeds_stay_in_the_c7a_band` (crate ai, release, `--ignored`) : trésor moyen France
34 286 < 40 000 sur main ; 41 889 après RS-B (ae5d94f1).

## État
- [ ] Bissection sur les fusions de première parenté ae5d94f1..main (arbres extraits par `git archive` dans le scratchpad).
- [ ] Diagnostic.
- [ ] Décision (correctif ou recalibrage + ADR).
- [ ] century_probe normale 10 graines.

## Prochaine étape
Lancer la bissection.
