# tx-m2-ville-prise (ADR 0723)
Spec : ~/dev/rea-tww3/docs/wip/integration-cent-ans.md, fiche M2. Branche `tx/m2-ville-prise`.

## État
- [x] ADR 0723, données (capture.json + schéma + data-model) : foreign_culture_extra, banditry, gift, liberate, scores IA
- [ ] (a) culture étrangère dans occupation_unrest / preview
- [ ] (c) banditisme (CaptureState::banditry, échéance dans resolve_unanswered)
- [ ] (b) Gift / Liberate : enum, recipient dans l'ordre, ai_choice, grudges::forget_settlement
- [ ] vues (CaptureOptionView.recipient), seed-bots, client, tests tx_m2_*

## Prochaine étape
Brancher (a) et (c) dans capture.rs.
