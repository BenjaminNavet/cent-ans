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

### Résultats (sonde release, moyenne 8 graines ; budget France cumulé sur 50 tours)
| État de main | France | Sièges/t | Batailles | Recettes | Armées | Admin. | Autre | Échec |
|---|---|---|---|---|---|---|---|---|
| ae5d94f1 RS-B seul | 41 889 | 1,40 | 52 | 1 732 989 | 750 791 | 603 925 | +51 988 | — |
| 45c79468 RS-B + TW2 SB-T3 | — | — | — | — | — | — | — | ne compile pas (corrigé par 38954b4f) |
| 38954b4f idem + correctif | 34 286 | 1,41 | 61 | 1 705 305 | 695 803 | 590 156 | −7 802 | trésor |
| a2672d2e + FE vague 1 | 19 482 | — | — | 1 545 331 | 634 118 | 510 868 | −18 687 | banqueroute graine 5 (×3) |
| 0cdafa9c + FE vague 2 | 16 601 | 2,70 | 82 | — | — | — | — | trésor |
| b00e1e98 + TW2 T5 | 21 141 | — | — | 1 590 851 | 682 864 | 525 113 | −12 152 | banqueroute graine 3 |
| 5aae540f main (RS-C) | 24 336 | 2,44 | 80 | 1 613 028 | 688 067 | 531 583 | −18 314 | trésor |

Le 34 286 cité pour main est la valeur de 38954b4f ; main réelle (5aae540f) = 24 336.
Reconstitution payée par la France : ~5 700 livres / partie seulement.

## Prochaine étape
Diagnostic 2 (`rs_m_diag2.rs`) : provinces de la France au départ / à mi-partie / à la fin et à qui
elles passent, sur 38954b4f et main.
