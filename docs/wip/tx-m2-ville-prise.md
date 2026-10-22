# tx-m2-ville-prise (ADR 0723)
Spec : ~/dev/rea-tww3/docs/wip/integration-cent-ans.md, fiche M2. Branche `tx/m2-ville-prise`.

## État
- [x] ADR 0723, donnees (capture.json + schema + data-model)
- [x] (a) culture etrangere (occupation_unrest, preview)
- [x] (c) banditisme (CaptureState::banditry, echeance dans resolve_unanswered)
- [x] (b) Gift / Liberate (enum, recipient dans l'ordre, ai_choice, grudges::forget_settlement), client (recipient), seed-bots
- [x] tests tx_m2_* (capture_tests.rs), fmt, clippy workspace + client OK

## Connu
Echecs deja presents sur main, independants : campaign::save_load_round_trip, m5::save_round_trip_keeps_diplomacy,
c5_trade::trade_state_survives_saves, m3_sequential_turn::ai_factions_play_one_after_the_other_in_id_order.

## Reste
Pas de choix de beneficiaire dans l'UI (le defaut propose est utilise) ; pas de pretendant pour Liberer ;
pas de sondes de revoltes rejouees (seed-bots).
