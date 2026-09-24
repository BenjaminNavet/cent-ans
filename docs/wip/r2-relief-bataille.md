# Lot R2 — Relief réaliste des champs de bataille

Branche : `worktree-agent-a3ec0644f24b4ad29` (non fusionnée). ADR : `docs/decisions/0020-relief-des-champs-de-bataille.md`.

## État
- [x] Squelette : `core/crates/sim-battle/src/relief.rs` (vide), tirages de référence d'avant R2 relevés.
- [x] Cœur : `relief.rs` (fBm + déformation de domaine, ridged, vallons, talus/escarpements,
  micro-relief, plaine alluviale, « replat » des lignes de déploiement, rampe du défenseur) ;
  bois/boue en grappes (`forest_parts`, `mud_parts`) ; `site.rs` (`Occupied.parts`) ; pont
  (`forests`/`mud` = ancres + parties).
- [x] Tests `core/crates/sim-battle/tests/relief.rs` (7) ; `ai.rs` (graines 1/4/6), `b6.rs`
  (empreintes), `f5.rs` (contact cherché pendant la course) adaptés — voir points ouverts.
- [x] Captures « avant » `docs/img/r2/avant_*.png` (dylib d'avant copiée dans le scratchpad).
- [ ] Rendu : carte de relief (pente, creux, crêtes), normales de détail, couleurs, horizon, lisières.
- [ ] Captures avant/après `docs/img/r2/`, FPS.
- [ ] ADR 0020.

## Prochaine étape
Rendu Godot (carte de relief, shader de sol, arbres sans double comptage, horizon), puis captures « après » et FPS.
