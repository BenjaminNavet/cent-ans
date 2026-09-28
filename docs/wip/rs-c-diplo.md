# RS-C — plafond d'opinion par motif et démolition (cœur, IA)

Branche `feat/rs-c-diplo` (worktree d'agent, partie de main 48f1a5f8). Orchestration : `docs/wip/restes.md`.
Cible cargo privée : `core/target-rs-c` (à supprimer en fin de lot). ADR : 0108 (0104-0106 réservés GA, 0107 pris).

## Points traités
1. Opinion « Mariage entre nos maisons » empilée sans plafond (+150), ambassades de héraut
   (6 à la fois) — `docs/wip/eq6-guerre-toutes-difficultes.md`.
2. L'IA sans ordre de démolition : Suisses en banqueroute (2,0/déc., graine difficile) —
   `docs/wip/eq5-ia-banqueroutes-intrusions.md`.

## Conception
- `data/rules/diplomacy.json` (+ `diplomacy_rules.schema.json`, `DiplomacyRules`, `OpinionMotive`) :
  `opinion_caps` par motif (`marriage` 30, `herald_embassy` 20 ; `gift`, `treaty` possibles).
  `CampaignState::add_capped_modifier` : le nouveau modificateur est rogné à ce qui reste sous
  le plafond ; à plafond, il ne fait que prolonger les modificateurs en cours. Appliqué à l'ajout
  seulement (pas à la lecture) : le test EQ6 `marriages_do_not_stop_the_claim_war` (3 mariages
  poussés à la main) reste valable, et les vieilles sauvegardes expirent d'elles-mêmes (80 tours).
- `Order::Demolish { settlement, building }` (`{"type": "demolish", ...}` au pont, sans UI) :
  rase le bâtiment (toute sa chaîne), rend `economy.json` `demolition_refund_percent` (10 %) du
  coût en argent ; refusé sous siège ou si un autre bâtiment (ou le chantier) en dépend
  (`buildings::demolition_blocker`).
- `FactionState::deficit_seasons` (saisons de déficit d'affilée, `resolve_economy`).
- IA : `economy.json` `ai_demolition` (4 saisons de déficit, entretien des bâtiments qui ne se
  paient pas > 40 % du revenu brut, 1 par saison ; construction plafonnée à 30 % : hystérésis).

## État
- [x] Squelette : données, schémas, pytest, `DiplomacyRules`, `add_capped_modifier` branché
  (mariage, héraut ×2, présents, traité), `Order::Demolish`, `deficit_seasons`.
- [x] IA : `plan_demolitions` dans `ai::campaign::plan_economy`.
- [x] Tests : `sim-campaign/src/rs_c_tests.rs` (10 : plafonds, renouvellement, négatif, sans
  plafond, démolition, refus, JSON du pont, `deficit_seasons`), `ai/tests/rs_c_demolition.rs` (3).
- [x] ADR 0108 ; codex `cdx_jeu_diplomatie`, `cdx_jeu_construction` (une phrase chacun).
- [x] Mesures (binaires release dans le scratchpad ; base = main 48f1a5f8).
- [x] fmt, clippy `--workspace --all-targets -D warnings`, `cargo test --workspace` (0 échec),
  pytest 890 passés / 2 ignorés.

## Mesures
`balance_probe campaign 200` (normale, graines 1-8 | 9-16) :
| Variante | Guerre FR-EN | Révoltes / partie | Banqueroutes / fac. / déc. | Impôt Haut | Mécontent. moyen |
|---|---|---|---|---|---|
| base (main) | 70 % / 64 % | 5,0 / 7,4 (6,2) | 0,08 / 0,08 | 32 / 33 % | 20,7 / 20,4 |
| **RS-C** | 69 % / 59 % | 5,6 / 6,1 (5,9) | 0,08 / 0,08 | 33 / 31 % | 21,1 / 19,4 |

`century_probe 464` (guerre FR-EN moy. [min-max], graines dans 55-75, trêves, révoltes / 200 t.,
banqueroutes / fac. / déc.) :
| Niveau | Base (main) | RS-C |
|---|---|---|
| Facile (5) | 68 % [56-75], 5/5, 11,6, 4,4, 0,114 | 70 % [63-76], 4/5, 12,4, 9,0, **0,084** |
| Normale (10) | 66 % [59-70], 10/10, 14,1, 6,3, 0,077 | 67 % [55-72], 10/10, 13,4, 5,4, **0,047** |
| Difficile (10) | 64 % [57-71], 10/10, 14,6, 5,7, 0,059 | 65 % [60-69], 10/10, 15,3, 5,6, **0,039** |
| Très difficile (5) | 59 % [45-69], 4/5, 14,8, 7,2, 0,044 | 57 % [54-61], 3/5, 15,2, 8,0, 0,052 |

Pire faction par graine (banqueroutes / déc.) : base jusqu'à 3,4 (Écosse, facile) et 3,0 (normale),
Suisses 1,0 (normale g6) ; RS-C au plus 1,8 (Écosse, difficile g4), Suisses 0,4 à 1,0.
Critères : guerre moyenne 55-75 % tenue aux quatre niveaux ; révoltes dans 4-10 partout ;
banqueroutes en baisse sur trois niveaux sur quatre (très difficile +0,008, dans le bruit).

Sonde release `m3_grid_ai::fifty_turns_on_eight_seeds_stay_in_the_c7a_band` : **rouge dès main**
(trésor moyen de la France 34 286 < 40 000 ; sièges 1,407). RS-C : France 39 450, Angleterre 27 465,
sièges 1,470, bloquées 0,09, débarquements 4,2, batailles 58,4 — mieux, mais encore 550 livres sous
le plancher. Le plancher n'a pas été touché (hors lot).

## Points ouverts
- Sonde c7a rouge sur main (trésor de la France) : à recalibrer ou à corriger par le lot fautif.
- Pas de bouton « Raser » dans l'UI (commande disponible au pont : `{"type": "demolish",
  "settlement": ..., "building": ...}`).
- L'Écosse reste la faction la plus souvent en banqueroute (armées, pas bâtiments).

## Prochaine étape
Lot terminé. Fusion par l'orchestrateur ; `core/target-rs-c` supprimé.
