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
- [ ] IA : ordre de démolition en déficit prolongé (`ai::campaign::plan_economy`).
- [ ] Tests Rust : plafond (mariage, héraut, renouvellement), démolition (remboursement, refus,
  dépendance), IA qui rase.
- [ ] ADR 0108, codex si besoin.
- [ ] Mesures avant/après (balance_probe 1-16, century_probe 4 niveaux, sonde c7a).

## Prochaine étape
Plan de démolition de l'IA, puis tests.
