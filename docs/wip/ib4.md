# IB4 — liens `ib:`, bulles filles riches et de règle, placement — fichier de reprise

Branche `feat/ib4-bubbles` (depuis `main` ee0a8816). Spec IB § 3.3-3.4, ADR 0109, orchestration
`docs/wip/ib.md`. Fichiers : `codex_text.gd`, `codex_bubbles.gd`, `rich_tooltip.gd` (section IB4
en fin de fichier + `_icon_text`, `cost_text`, `_effects_block`, `population_class`),
`tooltip_view.gd`, `data/ui/tooltips.json`, `ib_chain_test.gd`, `tools/tests/test_tooltip_schemas.py`.

## État

- [x] 1. `CodexText.ib_link` / `ib_key` / `mark_ib_read` (souligné tant que non ouvert)
- [x] 2. `RichTooltip` : `rule_entry`, `rule_link`, `entity_link`, `link_spec`, `rule_spec` ;
  noms d'entités (prérequis, habilitants, ressources, déblocages) en liens ; libellés d'effets,
  stats, vedettes en liens `ib:rule:` (rendu `TooltipView`) ; textes `tooltips.json`
  (effects 39+, stats 9, gauges 10) ; pytest de couverture
- [x] 4. Défauts IB1 : icône de vedette « Tir »/« Mêlée » (repli `battle_state_*`), ambiance
  en police italique explicite et encre atténuée
- [ ] 1b. `CodexBubbles` ouvre les liens `ib:` (entité → `TooltipView.build(link_spec, true)`,
  règle → bulle `rule`), clic gauche → fiche `entry_for_entity`
- [ ] 3. Placement latéral, fil d'Ariane, réduction des ancêtres
- [ ] 5. `ib_chain_test` étendu

## Prochaine étape

`codex_bubbles.gd` : clés de lien génériques (`link_key`), `open` pour `ib:`, placement.
