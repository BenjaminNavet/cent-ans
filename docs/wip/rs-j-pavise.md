# RS-J — Pavois face à une cible cachée

Branche `feat/rs-j-pavise` (depuis `main`). Chantier RS (`docs/wip/restes.md`), point ouvert de
RS-D (`docs/wip/revue-code.md`, n° 2 : les pavois attendent une cible cachée).

## État
- [x] Squelette : test ignoré `pavised_crossbowmen_close_in_on_a_hidden_target` repris de
  `feat/rs-d-trade` dans `core/crates/sim-battle/tests/review_fixes.rs`.
- [ ] Sondes avant.
- [ ] Correction de la règle (`start_attack` + boucle de mouvement, `sim-battle`).
- [ ] Test activé + non-régression cible visible.
- [ ] Sondes après, fmt/clippy/test, merge main.

## Prochaine étape
Sondes avant (release) : ep7_historical `survey_all`, ep9b_duel `survey`,
`ai_beats_a_passive_ai_at_equal_forces`, eq7_cavalry `probe_*`.
