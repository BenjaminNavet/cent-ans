# WH econ — état

Branche `wh/econ` (worktree `../gp-wh-econ`). Spec : `docs/wip/wh/economie.md` §3 points 1, 2, 3, 4, 5, 7.

- [x] 1 ordre public décomposé (`population::unrest_breakdown`, pont `unrest_terms`, infobulle de jauge)
- [x] 2 revenus par source (`income_breakdown.rs`, ponts `get_income_breakdown`, `income_lines`, UI budget + infobulle colonie)
- [x] 3 impôt par province (`province_tax.rs`, `Order::SetProvinceTax`, `ProvinceTaxSection`, passe IA dans `ai/campaign/economy.rs`)
- [x] 4 édits : `cost` (livres/saison dans « Cour et administration », prestige une fois), `requires`, 8 nouveaux édits
- [x] 7 bâtiments : 4 de cité (hôtel de ville, cour du bailli, halle aux grains, prévôté) + 4 de colonies mineures
- [~] 5 plafond d'emplacements `building_slot_cap` (data/settlements/rules.json) ; sonde campaign_probe avant/après à faire
- [ ] tests Rust (édits, plafond), test headless UI, ADR 0274/0275, lots.md
