# EQ7 — la cavalerie attend son infanterie sous les flèches

Suite de l'ADR 0052 (panique des chevaux). Branche `feat/eq7-cavalry-waits`, worktree `../gp-eq7`.

## But
L'IA d'un assaillant ne lance plus sa cavalerie à portée d'archers pourvus de flèches avant
l'arrivée de son infanterie (ou la fait passer par le flanc). Rendre aux Français une partie
des victoires perdues dans la bataille mixte `b6` (30-31/64 depuis ADR 0052 / EP11).

## État
- [x] worktree, sonde `tests/eq7_cavalry.rs` (`probe_mixed_battle`, `EQ7_SEED=n` pour la trace)
- [ ] mesure de référence
- [ ] règle dans `plan_horse` (`src/ai.rs`)
- [ ] mesures : b6 64 graines, R4 (`R4_FRENCH=heavy R4_JITTER=1`), R2b active/passive
- [ ] empreintes des tests, ADR, fusion

## Prochaine étape
Mesure de référence puis trace d'une graine perdue.
