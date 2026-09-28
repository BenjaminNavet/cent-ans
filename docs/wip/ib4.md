# IB4 — liens `ib:`, bulles filles riches et de règle, placement — fichier de reprise

Branche `feat/ib4-bubbles` (depuis `main` ee0a8816, `main` 0852a64f fusionné). Spec IB § 3.3-3.4,
ADR 0109, orchestration `docs/wip/ib.md`. Fichiers : `codex_text.gd`, `codex_bubbles.gd`,
`rich_tooltip.gd` (section IB4 en fin de fichier + `_icon_text`, `cost_text`, `_effects_block`,
`population_class`), `tooltip_view.gd`, `data/ui/tooltips.json`, `ib_chain_test.gd`,
`tools/tests/test_tooltip_schemas.py`.

## État : terminé, prêt à fusionner dans `integration/ib`

- `CodexText.ib_link` / `ib_key` / `mark_ib_read` : liens `ib:<kind>:<id>` en couleur de mot-clé,
  soulignés tant que la bulle n'a pas été ouverte (entité : ou tant que sa fiche n'est pas lue).
- `RichTooltip` (section IB4) : `rule_entry`, `rule_link`, `link_rule_label`, `entity_link`,
  `link_spec` (règle → `rule_spec` ; entité → `spec_for`, sinon spec simple tirée du BBCode pour
  ressources, traits, compétences), `rule_spec`. Noms d'entités (prérequis, habilitants,
  déblocages, ressources des coûts) en liens `ib:` ; libellés d'effets des blocs BBCode en liens.
- `TooltipView` : libellés d'effets, de stats et légendes des vedettes en liens `ib:rule:` ;
  vedette sans icône (« Tir », « Mêlée ») → `battle_state_shoot` / `battle_state_melee` ;
  ambiance : police italique du thème posée en police normale + encre `FLAVOUR_COLOR`.
- `data/ui/tooltips.json` : `effects` (toutes les clés d'`EFFECT_LABELS`), `stats` (9), `gauges`
  (copie de `GAUGE_TEXTS`). Textes vérifiés sur `core/` (descriptifs quand le mécanisme n'est
  pas lu directement : prestige, piété, diplomatie, intrigue, fécondité, résistance aux sièges).
- `CodexBubbles` : `open` accepte les clés `ib:` (vue détaillée `TooltipView.build(spec, true)`),
  `link_key` / `link_meta`, clic gauche → `entry_for_spec` ; placement `place_beside` (droite,
  gauche, dessous, glissement sous une ancêtre), alignement sur la ligne du mot-clé
  (`keyword_y`), réduction des ancêtres anciennes (`set_collapsed`, clic pour rouvrir), fil
  d'Ariane (`_refresh_breadcrumbs`, `back_to`), `area_override` pour les tests ; la bulle suit sa
  taille minimale (`minimum_size_changed` → `reset_size`). Le pied de la vue adoptée garde coût /
  entretien / durée (seule l'aide de touche part).
- Tests : `ib_chain_test` (IB3 + IB4), `ib_layout_test`, `p2c_ui_test`, `po_ui_test`, pytest.

## Points ouverts

- Alignement sur la ligne du mot-clé non testé en headless (souris et mise en page réelles) ;
  contrôle visuel à faire par l'orchestrateur (chaîne de 4 bulles à 1280 × 720).
- Une ancêtre rouverte (clic) reprend sa place sans re-placement de ses descendantes.
- Clés de règle partagées (`morale` stat / jauge) : ordre de recherche effects → stats → gauges.
