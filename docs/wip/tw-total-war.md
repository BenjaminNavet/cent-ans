# TW — rapprocher Cent Ans de Total War (nuit 10-10)

Demande joueur (10-10) : orchestrateur Opus, critiques Sonnet qui comparent le jeu à Total War et listent les écarts, puis corrections en autonomie toute la nuit.
Mandat : **tout le jeu** (campagne + batailles 3D) ; références **Medieval II + Warhammer III** ; crédits cloud **≤ 5 $** (docs/budget.md) ; fusion dans main **et push autorisés** après tests verts ; pas de 3D navale.
Sessions parallèles : UX5 (5 écrans, `docs/wip/ux5-ecrans.md`), WR (restes WH, `docs/wip/wr-restes-wh.md`, ADR 030x) : ne pas toucher leurs lots. ADR TW à partir de **0320**.

## État
- [x] Vague critique (6 Sonnet ; brief `docs/wip/tw/brief-critique.md`) → `docs/wip/tw/<rôle>.md`
  rôles : bataille-controles, bataille-simulation, bataille-ia-sieges, bataille-ressenti, campagne-m2, transitions
- [x] Synthèse + lots → `docs/wip/tw/lots.md`
- [ ] Vagues de corrections (worktrees `../gp-tw-<lot>`, branches `tw/<lot>`, ff-only)
- [ ] Vérif finale (fmt/clippy/test/pytest/smoke) + push

## Prochaine étape
- Intégration 1 (bsim, pursuit, retreat, m2a, m2b) : main + push (f02e5e743).
- Intégration 2 (ai-deploy, bctrl, bfeel) : main (b3b4dc965), push après tests Godot.
- En cours : balance, trans, misc-camp, misc-ui (depuis main) ; siege, reinf (basés sur `wr/int` : ne reprendre que les commits TW après fusion WR).
- Ensuite : lot oeil (≤ 10 captures headless), vérif finale, mémoire.
