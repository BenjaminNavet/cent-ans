# WIP — B3 Mécaniques de bataille, siège et guerre navale (Codex « Le jeu »)

Conception : `docs/design/2026-09-25-bulles-partout.md` (lot B3). Suivi général : `docs/wip/bulles-partout.md`.

## Périmètre
Fiches `category: "mecanique"` (ids `cdx_jeu_…`) sur la bataille 3D, l'auto-résolution, les sièges
(campagne et assaut 3D), les incendies, les engins, la guerre navale et le débarquement ; une fiche par
ordre du chef (`entity: "order_…"`) ; `gameplay` ajouté à `cdx_deroute_debandade` ; liens `[[cdx_…]]`
dans les descriptions de `data/battle_orders/`.

Chiffres : tirés de `core/crates/sim-battle/src/` (sim.rs, impact.rs, unit.rs, site.rs, siege*.rs,
orders.rs, naval/), `core/crates/sim-campaign/src/` (battle_auto.rs, siege.rs, naval.rs, movement.rs,
battle_request.rs, battle_forecast.rs, dynasty.rs), `data/rules/*.json`, `data/naval/rules.json`.

Liens vers des fiches d'unités de B5 pas encore écrites : `data/codex/_b3_links.md` (ids probables).

## État
- [x] Squelette (ce fichier, `_b3_links.md`)
- [x] Fiches bataille (moral, fatigue, formations, déploiement, charge, pieux, mêlée, tir, flancs, terrain, relief, météo, général, résolution auto, pertes et prisonniers)
- [ ] Fiches siège (faites : siège, murailles, assaut, bélier, sortie ; restent engins, huile, incendies)
- [ ] Fiches navales
- [ ] Fiches des ordres + liens dans `data/battle_orders`
- [ ] Validateur Codex + pytest verts

## Prochaine étape
Fiches siège restantes (cdx_jeu_engins_siege, cdx_jeu_huile_bouillante, cdx_jeu_incendies), puis naval, puis ordres. Les liens vers des cdx_jeu_ pas encore écrits font échouer le validateur jusqu'à la fin du lot.
