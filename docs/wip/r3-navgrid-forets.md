# R3 — forêts historiques (R1) dans la grille de navigation

Branche : `worktree-agent-aa0218e94ad47cabc` (depuis main afd327d4). ADR : `docs/decisions/0045-forets-historiques-dans-la-grille.md`
(chiffres avant/après dans l'ADR).

## État
- [x] Squelette (ce fichier, ADR stub)
- [x] Mesures « avant » (sonde `core/crates/ai/examples/r3_route_probe.rs`, `march_range_probe`, `m3_grid_ai` ignoré)
- [x] `navgrid.py` : forêt de `splat.png` R1 (moyenne 2 × 2), roche/lande = montagne sur collines seulement,
      marais de `wetlands.png` (R ou G ≥ 0,5) ; `navgrid_splat.png` supprimé
- [x] `navgrid.png` + aperçu régénérés ; tests Python (4 nouveaux tests unitaires) verts
- [x] Mesures « après » (m3_grid_ai 50 tours × 8 graines vert), ADR 0045, note dans ADR 0019
- [ ] `cargo test` complet, pytest complet, build.sh + smoke, fusion de main

## Prochaine étape
Suite de tests complète, puis `git merge main` et revérification.
