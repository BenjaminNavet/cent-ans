# ECO2 — le roi et sa dette

Lot : refonte économique de la campagne (juge it.3 chantiers 1 et 3). ADR 0777 (finances royales : charges, crédit, impôts exceptionnels, banqueroute) et 0778 (éco-guerre, budget lisible).

## Plan
- Données : `data/rules/finance.json` + `data/schemas/finance_rules.schema.json` ; `data-model::FinanceRules`.
- Cœur : `sim-campaign/src/finance.rs` (maison du roi, gages, sièges, obligations, prêts, banqueroute royale, impôts exceptionnels, IA).
- Ordres : `Borrow`, `Repay`, `LevyExtraordinary` ; `SetCoinage` existant journalisé.
- Journal des causes : `RoyalDefault`, `ExtraordinaryTax`, `Debasement` (cause_builders).
- game-api : budget par poste (revenus/dépenses) dans la vue économie.
- Mesures : `campaign_probe --turns 40 --seeds 1,2 --passive --full` ; seed bots `--quick`.

## État
- [ ] squelette
