# Lot R2b — l'IA de bataille lit le relief — en cours

Branche : `worktree-agent-a0839ed779ec029d6`. Suite de R2 (`docs/wip/r2-relief-bataille.md`, ADR 0020).

## Mesure
`cd core && cargo test --release -p sim-battle --test ai_relief -- --ignored --nocapture`
(`survey_active_against_passive` : IA active contre camp passif, armées miroir, graines 0-15 × 2 camps
= 32 batailles par terrain ; `trace_one_battle` avec `R2B_TRACE=plains,0,defender` pour suivre une
bataille).

| Terrain | avant R2 (c5eb8861) | après R2 (main, 57032d45) | après R2b |
|---|---|---|---|
| plaine | 21/32 | 17/32 | — |
| bocage | 11/32 | 9/32 | — |
| collines | 14/32 | 16/32 | — |
| montagne | 14/32 | 18/32 | — |

## État
- [x] Mesure (`tests/ai_relief.rs`).
- [x] Squelette `core/crates/sim-battle/src/relief_ai.rs` (`ReliefMap` : proéminence locale, pente,
  montée, coût de marche, ligne de vue, contre-pente), cache `BattleSim::relief_map` (`OnceCell`,
  remis à zéro par `field_mut`).
- [ ] Usages dans `ai.rs` : position défensive (crête / contre-pente), tireurs (vue + hauteur),
  avance de l'attaquant (contourne les pentes fortes), pas de charge en montée raide.
- [ ] `tests/ai.rs` : graines 0-2 d'origine + test statistique.

## Prochaine étape
Brancher `ReliefMap` dans `ai.rs`, mesurer, itérer.
