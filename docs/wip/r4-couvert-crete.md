# Lot R4 — contre-pente contre l'arc long, position « couvert × crête »

Branche `worktree-agent-a87b717776dc51d1c`. Suite de R2b (`docs/wip/r2b-ia-relief.md`), ADR 0046.

## Référence (main afd327d4, avant R4)
Mesure R2b, graines 0-63 : plaine 113/128, bocage 89/128, collines 98/128, montagne 82/128 (total 382/512).

## État
- [x] Squelette : `data/rules/missile_arc.json` + schéma + test Python ; `src/missile_arc.rs`
  (`MissileArcRules`, `FireMode`, `arc_clears`) ; `src/sim/indirect.rs` (`fire_mode`, observateur) ;
  `Unit::seen_at` ; `ShotEvent::indirect` + pont `get_shots().indirect` ; `src/position.rs` (vide).
- [ ] Tir indirect branché et mesuré.
- [ ] Score de position défensive + branchement IA (B6/R2b), attaquant qui vise le point faible.
- [ ] Attaquant plus haut qui attend (borné dans le temps).
- [ ] Tests `tests/r4.rs`, mesures, ADR 0046.

## Prochaine étape
Compiler, commiter le squelette, puis tests du tir indirect.
