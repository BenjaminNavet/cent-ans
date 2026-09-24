# Lot R2 — Relief réaliste des champs de bataille

Branche : `worktree-agent-a3ec0644f24b4ad29` (non fusionnée). ADR : `docs/decisions/0020-relief-des-champs-de-bataille.md`.

## État
- [x] Squelette : `core/crates/sim-battle/src/relief.rs` (vide), tirages de référence d'avant R2 relevés.
- [ ] Cœur : bruit (fBm, déformation de domaine, ridged), vallons, ruptures de pente, micro-relief,
  lignes de déploiement bornées, avantage du défenseur, bois/boue en grappes irrégulières.
- [ ] Tests `core/crates/sim-battle/tests/relief.rs`.
- [ ] Rendu : carte de relief (pente, creux, crêtes), normales de détail, couleurs, horizon, lisières.
- [ ] Captures avant/après `docs/img/r2/`, FPS.
- [ ] ADR 0020.

## Prochaine étape
Implémenter `relief.rs`.
