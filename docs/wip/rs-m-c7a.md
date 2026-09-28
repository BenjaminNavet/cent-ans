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

## Bissection
RS-B (ae5d94f1) ne contenait pas TW2 : sa mesure 41 889 est antérieure à SB/T1-T3. Points testés =
états successifs de main (reflog) : ae5d94f1 (réf.), 45c79468 (RS-B + TW2 SB/T1/T2/T3), a2672d2e
(FE vague 1), 0cdafa9c (FE vague 2), b00e1e98 (TW2 T5), 5aae540f (RS-C).
Script : `scratchpad/probe.sh <commits>` (git archive → build release, sonde + diagnostic
`rs_m_diag.rs` qui cumule le budget de la France ligne à ligne sur les 8 graines).

## Prochaine étape
Lire les résultats de la bissection.
