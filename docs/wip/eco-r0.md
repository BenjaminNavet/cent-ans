# ECO-R0 — correctifs et mesure (boucle juge 2)

Spec : `docs/audits/2026-10-29-economie-politique.md`, section « R0 ».

## État
- R0-a : `core/crates/ai/examples/eco_series.rs` (sonde, options `--seeds --turns --passive --factions`,
  sortie JSON lines) ; cinq métriques dans `core/crates/seed-bots/src/metrics.rs`
  (`army_spend_share`, `extraordinary_income_share`, `men_under_arms_peak`, `ransom_paid_max_seasons`,
  `provinces_controlled_end`). Bandes : à initialiser sur la mesure (voir plus bas).
- R0-b : `truce_turns` + libellé (`negotiation::truce_label`), `FactionState::truce_started`
  + `perjury::truce_age`, contrôle cumulé or/tribut (`Deal::committed_funds`), tribut suspendu
  (`unpaid_tribute_*` en données), `EventEffect::Alliance` + `evt_auld_alliance.json`.
- Tests : `core/crates/sim-campaign/tests/diplomacy/eco_r0.rs`, test de métriques dans `metrics.rs`.

## Terminé (reprise)
Tests diplomatie (6) et métriques verts, `cargo test --workspace` vert, clippy -D warnings propre, passe rapide seed bots : 0 constat.

## Mesure « avant » (passe quick : 8 bots, 20 tours, graine 1, maison du bot)
- army_spend_share : 0,16 - 0,34 (miroir 0,30)
- extraordinary_income_share : 0 partout (aucune levée ni emprunt en 20 tours)
- men_under_arms_peak : 537 - 1983 (540 pour la plupart ; miroir/IA stratégique 1983)
- ransom_paid_max_seasons : 0 partout (aucune rançon en 20 tours)
- provinces_controlled_end : 10 - 11 (provinces_held_end 10 - 11, droit)
Bandes initialisées (provisoires, 20 tours seulement) : army 0,1-0,45 ; extraordinary 0-0,25 ; men 400-2500 ;
ransom 0-2 ; controlled 5-16 (quick) / 3-16 (longue). À recalibrer sur la passe longue / sonde séculaire.

## Points ouverts / écarts
- Le constat 1 de l'audit (« Trêve de deux ans » = 20 saisons) est inexact : la constante de `game-api`
  valait déjà 8 saisons (= 2 ans). La durée du menu passe en données à 8 (comportement inchangé).
- `provinces_held_end` comptait en fait le contrôle de fait ; il compte désormais la possession de droit
  (`owned_provinces`) comme la spec le décrit ; `provinces_controlled_end` prend le contrôle de fait.
