# NT6ab petites suites

Branche `feat/nt6ab-leftovers`. Termine.

## Fait
- 1 discours adverse : `BattleScene._start_enemy_speech`, `BattleSpeech.skipped` (passer = pas de discours adverse), meme reglage `--no-speech`, voix via `VoiceLines` si fichiers existants.
- 2 indicateur vise : `BattleSiege._update_target_marks` (champ `target` des unites).
- 3 surprime : pont `mercenary_premium(_last_turn)`, ligne `BudgetTable` hors solde.
- 4 en-tete de province compacte (`ProvincePanel._compact_header`), avis 3 lignes max (`UiLayout.TOAST_MAX_LINES`).
- Test `game/tests/nt6ab_test.gd` OK ; smoke, po_ui, q6_side_panel, q6_toasts, c5, bv3_check OK.

## Points ouverts
- fe_ui_test : 1 echec (cadrage du selecteur de carte), sans rapport a priori.
- q6_ui/q6_diplomacy/ui1_lettrine : erreurs de compilation `SimFacade` sous --script (a verifier sur main).
- La surprime n est pas dans le solde du budget (prelevee a part par le core) ; affichee en ligne separee.
- Verification visuelle non faite.
