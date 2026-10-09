# WH diploa — diplomatie lisible

Branche wh/diploa. Spec : docs/wip/wh/diplomatie.md § 3 points 1, 2, 3, 4, 10. ADR 0278.

## État
- Données (bandes d'attitude, retrait d'accès, seuil d'hésitation) + schéma : fait.
- Core : call_to_arms_forecast, reason_turns_left, DiplomacyEntry (bande, allies/enemies/vassals), RevokeMilitaryAccess, Rupture dans TreatyRecord : fait (compile).
## Prochaine étape
- Pont (get_diplomacy, evaluate_proposal guerre, historique), UI, tests Rust, smoke UI, ADR 0278.
