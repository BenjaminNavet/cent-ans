# WIP — H5 « Monnaie » + H6 « Chevalerie, rançons, ordres » (règles, données, pont)

Conception : `docs/design/2026-09-23-histoire-et-savoir.md` § 5.1-5.2. API : `docs/design/h5-h6-api.md`.
Branche : `worktree-agent-a5c437f998e41d92b`.

## Plan
1. [x] Squelette : `sim-campaign::{coinage, ransom, chivalry}` (non branchés), champs d'état
   `serde(default)` (`coinage`, `price_level`, rançons, ordre de chevalerie), `Order::{SetCoinage,
   PayRansom, SetRansomTerms, ReleaseOnParole, FoundChivalricOrder}`.
2. [x] H5 règles : seigneuriage, inflation, multiplicateur de coûts, refonte, états (bourgeois/clergé),
   paramètres en données (`data/rules/coinage.json` ou pratique du dépôt), IA.
3. [x] H6 rançons : calcul, paiement intégral / échéances, défaut, termes (argent, province, parole),
   souverain captif (mécontentement, régence), cohérence avec événements de rançon.
4. [ ] H6 ordres : `data/chivalric_orders/*.json` + schéma, fondation, membres, effets, effet
   d'événement `found_chivalric_order` (event_check), Jarretière/Étoile, malus d'effondrement.
5. [ ] Pont Godot (`campaign_sim_coinage.rs` ou équivalent) + `docs/design/h5-h6-api.md`.
6. [ ] Tests `sim-campaign/tests/h5_h6.rs`, fmt/clippy/test, pytest, build, import, smoke.

## État
Règles H5 (coinage.rs), H6 rançons (ransom.rs) et ordres (chivalry.rs) écrites et branchées (tour, IA, coûts, population, régence du souverain captif, effet d’événement). Paramètres en constantes Rust (pratique du dépôt). Reste : données `data/chivalric_orders`, schémas, événements Jarretière/Étoile, pont, doc API, tests.

## Prochaine étape
Données + schémas (chivalric_order.schema.json, common, event), puis pont, puis tests h5_h6.rs.
