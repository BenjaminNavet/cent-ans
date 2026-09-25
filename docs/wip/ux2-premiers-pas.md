# UX2 — Premiers pas (barre du haut, tutoriel U15, conseil « que faire maintenant »)

Branche : `worktree-agent-a109e2bd8dafc923f`. Plan d'ensemble : `docs/wip/ux-prise-en-main.md`.
Audit : `docs/audit/a3-ui.md` (C9, C13, U1, U2, lot U15). Captures : `docs/audit/captures/ux2/`.

## Fichiers
- Barre du haut : `game/scripts/map/map_ui.gd` (section « UX2 : libellés de la barre »),
  `game/scripts/map/chronicle_controller.gd` (bouton plus grisé).
- Tutoriel : `game/scripts/ui/tutorial.gd` (placement, surbrillance, sommaire, « Plus tard »),
  `game/scripts/map/tutorial_controller.gd` (`postpone`, `resume`, `jump_to`).
- Conseil : `data/ui/next_hints.json` + `data/schemas/next_hints.schema.json`,
  `game/scripts/ui/next_hint.gd` (choix), `game/scripts/ui/next_hint_card.gd` (encart),
  `game/scripts/map/next_hint_controller.gd` (état, actions), réglage `interface/next_hint`.
- Tests : `game/tests/ux2_test.gd` (« ux2 OK »), `tools/tests/test_next_hints_schema.py`.

## État
- [x] Squelette : données, schéma, `NextHint`, `NextHintCard`, `NextHintController`.
- [ ] Tutoriel : placement, surbrillance, sommaire, « Plus tard ».
- [ ] Barre du haut : libellés adaptatifs, Chronique, focus.
- [ ] Branchements : campaign_map (setup, stages `next_hint`, `tutorial_toc`), réglages, aide.
- [ ] Test ux2, captures avant/après, smoke, merge main.

## Prochaine étape
Réécrire `tutorial.gd` (placement `place_panel`, surbrillance dorée, sommaire, « Plus tard »).
