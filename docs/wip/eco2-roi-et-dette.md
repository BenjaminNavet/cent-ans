# ECO2 — le roi et sa dette

Lot : refonte économique de la campagne (juge it.3 chantiers 1 et 3). ADR 0777 (finances royales : charges, crédit, impôts exceptionnels, banqueroute) et 0778 (éco-guerre, budget lisible).

## État
- [x] Données : `data/rules/finance.json` + schéma ; `data_model::FinanceRules`.
- [x] Cœur `sim-campaign/src/finance.rs` : maison du roi, gages, sièges, obligations (dot, saisie), prêts (lombards, florentines), cote de crédit, banqueroute royale, impôts exceptionnels (aide, gabelle, fouage), IA (`ai_finance_orders`, branché dans `ai/src/campaign.rs`).
- [x] Ordres `Borrow`, `Repay`, `LevyExtraordinary` ; journal des causes (`royal_default`, `extraordinary_tax`, `debasement`).
- [x] Budget : `TurnBudget`, `BudgetRecord`, `budget_lines` ; `game-api` `EconomyView` + `FinanceView` (`FactionPanelView.finance`).
- [x] Client : lignes de budget et dette dans `campaign/panels/faction/view.rs` (pas de boutons d'ordres).
- [x] Ponction d'épargne resserrée (1, 2, 4 saisons) ; butin et chevauchée relevés.
- [x] Mesures : voir ADR 0778 (pic France passive 82,3 k / 80,3 k, soit 5,7 / 5,6 saisons du net initial ; banqueroutes 1 / 0). Seed bots `--quick` : 0 constat.
- [x] Vérification finale : fmt, clippy workspace, `cargo test --workspace --no-fail-fast` (39 binaires ok), pytest schémas, `cargo check` client, seed bots `--quick` (0 constat).

## Reste / limites
- Boutons client pour emprunter, rembourser, lever un impôt (UX2).
- `ObligationKind::Tribute` sans producteur (les tributs de traité restent dans `ledger.tributes`).
- Passe longue des seed bots non lancée.
