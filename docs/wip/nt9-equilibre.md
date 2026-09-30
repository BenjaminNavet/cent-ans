# NT9 — équilibre après N6/N7

Branche `feat/nt9-balance` (worktree agent). Lot NT9 de `docs/wip/nt.md`. ADR 0128 (à compléter).

## État
- [x] Sonde : `BATTLE_TRACE=1` sur `century_probe` imprime chaque ligne de bataille FR/EN comptée (décomposition par type)
- [x] Sortie de garnison comptée pour les missions (`siege::sortie` → `missions::note_battle_won`), test `a_won_sortie_counts_as_a_won_battle`
- [x] Bélier en résolution automatique : `auto_assault_bonus_percent` (20) dans `siege_engines.json`, `BattleContext.assault_bonus_percent`, `assault_odds` et prévision ; tests `a_ready_ram_helps_the_auto_resolved_assault`, `a_broken_gate_softens_the_walls`
- [ ] Mesure avant (f36689196) / main / branche, décomposition par type
- [ ] Cause de ×1,9 et correction
- [ ] `cv3_ai_stances` graine 1
- [ ] Garde-fous, fmt/clippy/test workspace, pytest, smoke

## Chiffres
(à venir)

## Prochaine étape
Lire `old-probe.txt` / `main-probe.txt` (scratchpad), classer les lignes `BT`.
