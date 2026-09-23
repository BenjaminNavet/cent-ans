# WIP — HUD de campagne (lot F10a, composants)

Branche : `worktree-agent-a74b133436188f172`.

## État : terminé (composants seuls, non intégrés)
- [x] Squelette : 4 scènes `game/scenes/ui/{army_strip,general_seal,end_turn_cluster,news_letters}.tscn`, scripts, `hud_style.gd`.
- [x] ArmyStrip (cartes pleines ou compactes sur deux rangs, noms sans coupure de mot, multisélection, séparer)
- [x] GeneralSeal (sceau de cire, écu ou portrait, rang, points, posture avec menu, ravitaillement, mouvement)
- [x] EndTurnCluster (cloche, éventail d'alertes groupées, décision bloquante, raccourci `campaign_end_turn`)
- [x] NewsLetters (lettres scellées, déplier / écarter, `news_from_event`)
- [x] Aperçu `game/tests/hud_preview.tscn` + captures `docs/img/hud-preview.png`, `hud-preview-1920.png`, `hud-preview-20units.png`
- [x] Test headless `game/tests/hud_components_test.gd` (OK)
- [x] Guide d'intégration `docs/design/hud-campagne.md`

## Prochaine étape
Intégration dans `map_ui` / `campaign_map` par l'orchestrateur (guide § 3), après fusion de F2 (icônes) et F3 (alertes).
