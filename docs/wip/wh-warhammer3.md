# WH — rapprocher Cent Ans de Total War: Warhammer III (nuit 10-09)

Demande joueur (10-09) : orchestrateur Opus, critiques Sonnet qui comparent le jeu à TWW3 et listent les écarts, puis corrections en autonomie toute la nuit.
Mandat : référence **Warhammer III** ; périmètre **carte de campagne, systèmes de campagne, UI/UX** (pas les batailles 3D) ; crédits cloud **≤ 5 $** (docs/budget.md) ; fusion dans main **et push autorisés** après tests verts ; pas de 3D navale.
Session parallèle RX active (worktrees `../gp-rx-*`, ADR jusqu'à 0258) : ne pas toucher ses lots (docs/wip/rx/lots.md). ADR WH à partir de **0270**.

## État
- [x] Vague critique faite (7 Sonnet : carte, armees, economie, personnages, diplomatie, tour, ui ; brief docs/wip/wh/brief-critique.md) → `docs/wip/wh/<rôle>.md`
- [x] Synthèse + lots → `docs/wip/wh/lots.md` (12 lots, 2 vagues)
- [~] Vague 1 lancée (idle, hover, armya, econ, chars, diploa ; agents cent-ans-mech) ; vague 2 ensuite (armyb, uicards, turn, diplob, charsb, mapb2)
- [ ] Vagues de corrections (worktrees `../gp-wh-<lot>`, branches `wh/<lot>`, ff-only)
- [ ] Vérif finale (fmt/clippy/test/pytest/smoke) + push
