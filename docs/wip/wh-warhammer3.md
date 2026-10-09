# WH — rapprocher Cent Ans de Total War: Warhammer III (nuit 10-09)

Demande joueur (10-09) : orchestrateur Opus, critiques Sonnet qui comparent le jeu à TWW3 et listent les écarts, puis corrections en autonomie toute la nuit.
Mandat : référence **Warhammer III** ; périmètre **carte de campagne, systèmes de campagne, UI/UX** (pas les batailles 3D) ; crédits cloud **≤ 5 $** (docs/budget.md) ; fusion dans main **et push autorisés** après tests verts ; pas de 3D navale.
Session parallèle RX active (worktrees `../gp-rx-*`, ADR jusqu'à 0258) : ne pas toucher ses lots (docs/wip/rx/lots.md). ADR WH à partir de **0270**.

## État
- [x] Vague critique faite (7 Sonnet : carte, armees, economie, personnages, diplomatie, tour, ui ; brief docs/wip/wh/brief-critique.md) → `docs/wip/wh/<rôle>.md`
- [x] Synthèse + lots → `docs/wip/wh/lots.md` (12 lots, 2 vagues)
- [~] FUSIONNÉS dans main : hover, idle, diploa, chars, mapb2, econ, turn, armya (bd06cd4e8). En cours : diplob, charsb, armyb, uicards (uicards doit aussi faire repasser po_ui_test, déjà rouge sur main : bandeau d'ost sur SIDE_PANEL, polices 14 px). cv3_ai_stances : graines 1, 3, 5 (balayage 1-8 refait après chaque lot qui décale le flux). Hors WH, rouge sur main : tools/tests/test_budget.py (ligne 0,049 $ d'une autre session).
- [ ] Vagues de corrections (worktrees `../gp-wh-<lot>`, branches `wh/<lot>`, ff-only)
- [ ] Vérif finale (fmt/clippy/test/pytest/smoke) + push

## Notes de fusion
- Worktree : après rebase, `godot --headless --path game --import` (nouveaux class_name) avant les tests ; `tools/run_godot_tests.sh` avec LOG_DIR dans le scratchpad.
