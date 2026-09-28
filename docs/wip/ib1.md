# IB1 — infobulles en sections (rendu § 2.1-2.3) — fichier de reprise

Branche `feat/ib1-layout` (worktree agent). Spec : `docs/superpowers/specs/2026-09-28-ib-infobulles-design.md`
§ 2.1-2.3, ADR 0109, orchestration `docs/wip/ib.md`.

## État

- [x] 1. `game/tests/ib_shot.gd` (fenêtré, `--mode=avant|apres`) ; planche `docs/img/ib/avant/` (5 vues)
- [x] 2. `RichTooltip.*_spec` (unités, bâtiments, techniques), `to_bbcode(spec)`, `spec_for(key, live)`
- [x] 3. `TooltipView.build(spec, detailed)` en sections
- [x] 4. Branchement recrutement / armée / `unit_card` / bâtiments / techniques
- [x] 5. `ib_layout_test.gd` activé
- [ ] 6. Planche `docs/img/ib/apres/`

## Choix

- `tooltip_text` = « ib:<kind>:<id> » + BBCode de repli sur les lignes suivantes (les tests de
  contenu existants et `make_panel` lisent le repli) ; `live` en métadonnée `ib_live`
  (`RichTooltip.set_tooltip`, `panel_for`). `RichButton`, `IconChip`, `RichPanel` passent par `panel_for`.
- `army_strip.gd` (cartes de régiment) : `_make_custom_tooltip` construit la spec directement.
- Sens favorable des effets (couleur) : `lower_is_better` ajouté à `tooltip_style.json` + schéma.
- Largeur `width_px` en unités d'interface : `content_scale_factor` applique l'échelle.
- `before`/`after` : lus dans `live["before_after"]` = {clé d'effet: [avant, après]} (IB5).

## Prochaine étape

Lancer smoke / po_ui / p2c_ui / hud_components ; planche après ; fusion de main.
