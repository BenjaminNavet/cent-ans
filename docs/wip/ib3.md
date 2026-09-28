# IB3 — chaîne de bulles à Alt maintenu — fichier de reprise

Branche `feat/ib3-chain` (depuis `main` c1775c8a, `main` a7d77308 fusionné). Spec § 3.1-3.2, ADR 0109.
Fichiers : `game/scripts/codex/codex_bubbles.gd`, `game/tests/ib_chain_test.gd`,
`game/scripts/ui/shortcut_sheet.gd` (ligne Alt). `tooltip_style.json` (bloc `chain`) en lecture.

## État : terminé, prêt à fusionner dans `integration/ib`

- `codex_bubbles.gd` : `chain_setting(key)` lit `chain` de `tooltip_style.json` (via `MapPaths`,
  constantes en repli) pour `hover_delay_s`, `idle_hover_delay_s`, `close_grace_s`, `max_bubbles`.
  Alt seul (`tooltip_explore`, ignoré avec Maj/Ctrl/Cmd, et Maj pendant Alt coupe la chaîne) →
  `explore_lock()` : fille en attente, bulle non épinglée sous la souris, infobulle native
  (`pin_native_tooltip`), contrôle survolé (`pin_control_tooltip`). Bulles « verrouillées par la
  chaîne » : méta `chain_locked` (pinned + chain), fermées à la grâce Alt relâché et souris hors
  bulles ; clic droit / T les épinglent durablement. `open(..., chain)` remplace la branche
  (`_trim_above(parent, true)`). Mot source surligné par `[bgcolor]` injecté dans le BBCode de la
  parente (appliqué hors survol d'un lien de l'étiquette, retiré à la fermeture de la fille).
- Version détaillée : `_detailed_view(text)` appelle `RichTooltip.spec_for(text)` (API attendue
  d'IB1, testée par `get_script_method_list`) puis `TooltipView.build(spec, true)` ; sinon BBCode.
  La vue est adoptée sans son cadre ni son pied (`open_view`).
- Tests : `ib_chain_test` OK, `p2c_ui_test` OK, smoke : seul échec connu FE (« 3 faction cards »),
  B1 bulles OK.

## Points ouverts (IB4)

- API attendue d'IB1 : `RichTooltip.spec_for(tooltip_text: String) -> Dictionary` (vide si pas de
  spec). Sans elle, les bulles verrouillées gardent le BBCode.
- Headless : fenêtre 64 × 64, `get_mouse_position()` reste (0,0) → le cas « chaîne gardée tant que
  la souris est sur une bulle » est sauté dans le test (message imprimé).
- Placement latéral, fil d'Ariane, réduction des ancêtres : IB4.
