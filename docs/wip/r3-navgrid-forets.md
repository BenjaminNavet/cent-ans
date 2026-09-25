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
- [x] main fusionnée (deux fois) ; `cargo fmt`/`clippy -D warnings`/`cargo test` verts ; pytest vert ;
      `core/build.sh` OK ; smoke : seul échec « music playlist too short » (campaign/war/court), venu du lot
      musique cb1b4416 fusionné dans main juste avant, sans lien avec la grille

## Points ouverts
- Prés humides nommés (Romney Marsh, Chat Moss, Bog of Allen, polders) : non ralentis (canal B mêlé aux prés de fond de vallée).
- Étangs jamais infranchissables (densité ≤ 0,75, pas d'emprise d'étang par case).
- Échec smoke musique à signaler au lot audio.

## Prochaine étape
Rendu du rapport ; fusion dans main par l'orchestrateur.
