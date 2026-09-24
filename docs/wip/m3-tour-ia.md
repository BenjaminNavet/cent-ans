# M3 — tour séquentiel et IA sur la grille (état)

Spec : `docs/design/2026-09-24-mouvement-libre.md` § 3.4, § 4, § 7. ADR : `docs/decisions/0010-free-army-movement.md`.
Branche : `m3-turn-ai` (worktree `agent-a2eb8f7f4bd84c3dd`), depuis `main` 93d466c (M1 + M2).

## État

- [x] Squelette : `data/ai/grid.json` + schéma `ai_grid.schema.json` + `AiGrid` (data-model) + test pytest.
- [ ] Tour séquentiel propre (`turn.rs`) : batailles IA contre joueur auto-résolues pendant le tour de l'IA.
- [ ] IA grille (`ai/src/grid.rs`) : attaque dans la bulle, évitement des ZdC plus fortes, `Embark`, étapes par tour, cache.
- [ ] `ai_minimal` : même exécution.
- [ ] Tests : 50 tours × 8 graines, déterminisme, performance.
- [ ] Fusion de main, fmt/clippy/test, build.sh, smoke.

## Mesures de référence (`settlements_probe 50 1..8`, release)

| Indicateur | avant M2 (1d199c4) | M2 (93d466c) | M3 |
|---|---|---|---|
| Trésor final FR / EN | 96 240 / 18 326 | 98 872 / 19 212 | |
| Δ provinces FR / EN | −0,4 / −0,1 | +0,9 / −0,5 | |
| Δ colonies FR / EN | −1,8 / −0,9 | +3,0 / −3,1 | |
| Écosse Δ colonies | −0,6 | −3,6 | |
| Sièges / tour (cités / autres) | 0,8 / 3,1 | 1,5 / 5,9 | |
| Prises / graine (cités / secondaires) | 10,1 / 65,8 | 10,6 / 93,5 | |
| Batailles / graine | 58,9 | **0** | |
| Armées bloquées / tour (niv. 4) | 0,3 (0,1) | 0,7 (0,5) | |
| s / graine | 6,8 | 15,4 | |

M2 : plus aucune bataille de campagne (les marches s'arrêtent dans la zone de contrôle, l'IA n'émet pas `Attack`).

## Prochaine étape

Implémenter `ai/src/grid.rs` puis le brancher dans `plan_armies`.
