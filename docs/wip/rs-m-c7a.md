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

## Diagnostic
- **TW2 SB-T3 (41 889 → 34 286)** : Philippe VI pris au combat dans plusieurs graines (12 échéances
  de rançon de 16 500 livres, ≈ 24 700 par partie ; aucune à ae5d94f1). Reconstitution ≈ 5 700 /
  partie, mercenaires achetés dans les ordres (total des ordres inchangé, −92 000).
- **FE (→ 24 336 sur main, 16 601-21 141 aux états intermédiaires, banqueroutes)** : chaque graine,
  l'Empire s'allie au Hainaut, déclare la guerre à la France vers t8 (« défense d'un allié ») et
  convoque l'ost : Autriche, Brabant, Gueldre, Hainaut, Confédérés, Milan, Savoie, Gênes, Vérone.
  Milan/Savoie prennent Lyonnais, Auvergne, Rouergue, Nîmois (cédés à la paix). Recettes −92 000.
- Contre-épreuve (ost d'Empire coupé, essai jetable) : trésor 25 065, sièges 1,95, batailles 62 ;
  recettes 1 745 719 mais entretien des armées +72 000 : l'IA dépense ce qu'elle regagne.
- Décision : pas de bogue ; plancher France 40 000 → 15 000 (ADR 0113). L'ost d'Empire → FE F8.

## Méthode
Script du scratchpad `probe.sh <commits>` : extraction de `core` et `data` du commit (archive) dans un
dossier, `touch` de tous les fichiers (sinon cargo réutilise les artefacts d'un autre arbre dans la
même cible), copie d'un test `rs_m_diag.rs` (budget de la France cumulé : ordres, recettes, armées,
bâtiments, admin., Table, autre ; `last_budget` après chaque `end_turn_with`), puis
`cargo test --release --no-fail-fast -p ai --test m3_grid_ai --test rs_m_diag -- --ignored --nocapture`.
Ne jamais partager une cible cargo entre deux arbres (le worktree a sa cible `target-rs-m-wt`).

## Century_probe 464 tours, normale, graines 1-10 (release, main 427b4978 = cœur de 5aae540f)
| Niveau | Guerre FR-EN moy. [min-max] | Graines 55-75 | Trêves | Révoltes / 200 t. | Banqueroutes / fac. / déc. | 1re faction fin |
|---|---|---|---|---|---|---|
| Normale (10) | 64 % [40-74] | 8/10 | 12,2 | 6,5 [1,3-13,8] | 1,09 [0,48-2,38] | 21 % |

Guerre et révoltes dans leurs bandes en moyenne (graines hors bande : 1 à 52 %, 6 à 40 %). Écart à
signaler : banqueroutes / faction / décennie 1,09 contre 0,05 à RS-B (ae5d94f1) — les petites
factions FE ; à juger en F8, hors lot.

## Prochaine étape
Sonde verte sur la branche, tests complets.
