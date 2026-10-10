# WH — rapprocher Cent Ans de Total War: Warhammer III (nuit 10-09)

Demande joueur (10-09) : orchestrateur Opus, critiques Sonnet qui comparent le jeu à TWW3 et listent les écarts, puis corrections en autonomie toute la nuit.
Mandat : référence **Warhammer III** ; périmètre **carte de campagne, systèmes de campagne, UI/UX** (pas les batailles 3D) ; crédits cloud **≤ 5 $** (docs/budget.md) ; fusion dans main **et push autorisés** après tests verts ; pas de 3D navale.
Session parallèle RX active (worktrees `../gp-rx-*`, ADR jusqu'à 0258) : ne pas toucher ses lots (docs/wip/rx/lots.md). ADR WH à partir de **0270**.

## État
- [x] Vague critique faite (7 Sonnet : carte, armees, economie, personnages, diplomatie, tour, ui ; brief docs/wip/wh/brief-critique.md) → `docs/wip/wh/<rôle>.md`
- [x] Synthèse + lots → `docs/wip/wh/lots.md` (12 lots, 2 vagues)
- [~] FUSIONNÉS dans main : hover, idle, diploa, chars, mapb2, econ, turn, armya, charsb (41bef9b8a). Rebasés, en vérification : armyb, diplob, uicards (uicards répare po_ui_test : C2 bandeau d'ost, C3 polices). cv3_ai_stances : graines 1, 3, 4, 5 (balayage 1-8 refait après chaque lot qui décale le flux). Icônes des 8 bâtiments econ (catalogue + vignettes tirées de miniatures existantes) et des 6 traits charsb (dessins voisins) : 0 $. Hors WH, rouge sur main : tools/tests/test_budget.py (ligne 0,049 $ d'une autre session), test_settlement_graph (graphe non connexe) et test_settlements_schema[prov_bar].
- Arbitrages : diplob garde la ligue comme filet de sécurité aux seuils actuels (0,055 des provinces, ×1,3 la puissance du second ; seuils bas = guerres ×3). uicards top3 (exécution des captifs) non fait : effet à spécifier.
- [ ] Vagues de corrections (worktrees `../gp-wh-<lot>`, branches `wh/<lot>`, ff-only)
- [ ] Vérif finale (fmt/clippy/test/pytest/smoke) + push

## Notes de fusion
- Worktree : après rebase, `godot --headless --path game --import` (nouveaux class_name) avant les tests ; `tools/run_godot_tests.sh` avec LOG_DIR dans le scratchpad.
