# PB3d — fin de tour dans un fil, rafraîchissement groupé

Branche `feat/pb3d-end-turn-thread` (worktree agent). Plan général : `docs/wip/pb3-performance.md`.
ADR : `docs/decisions/0081-fin-de-tour-dans-un-fil.md`.

## État
- [x] Pont Rust : `turn_job.rs` (fil de fin de tour, `resolve_turn` commun sync/async, test
  bit-à-bit), `campaign_sim_turn.rs` (`begin_end_turn`, `poll_end_turn`, `is_end_turn_pending`),
  garde `refuse_while_turn_pending` sur toutes les méthodes qui modifient l'état.
- [x] Pont Rust : `campaign_sim_provinces.rs` (`get_provinces_snapshot(ids)`,
  `get_settlements_live()` en Packed*Array).
- [ ] GDScript : `_on_end_turn` asynchrone + indicateur « Les cours d'Europe délibèrent… »,
  saisie désactivée pendant le calcul.
- [ ] GDScript : boucles `get_province_state` → instantané groupé, mises à jour seulement si changé.
- [ ] Bancs `pb1_turns.gd` et parcours `--journey --uncapped` (pire image pendant la fin de tour).
- [ ] Mesures avant/après (debug, release), ADR, fusion de `main`.

## Mesures
(à venir ; dylibs de référence construites depuis `main` dans le scratchpad)

## Prochaine étape
GDScript de la fin de tour asynchrone (`campaign_map.gd`).
