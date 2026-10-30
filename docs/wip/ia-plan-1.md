# IA-PLAN-1 — squelette du planificateur par divisions, plan « Tenir et recevoir »

Spec : `docs/audits/2026-10-29-bataille.md` (d) IA-PLAN-1. Branche : worktree-agent-a4262fad3109d43b2.

## État
- [x] Squelette `core/crates/sim-battle/src/ai/planner/{mod,plans,roles,division}.rs` (vide).
- [ ] Données `data/rules/battle_plans.json` + schéma ; drapeau `planner` dans `battle_ai.json`.
- [ ] Divisions, conditions, phases, branchement dans `ai::plan`.
- [ ] `RefusalLikely` (avant 300 s, banc `passive_attacker`).
- [ ] Assertion debug cible de division ; tests ; banc 20 graines ; ADR.

## Prochaine étape
Données et types.
