# WIP — B3 Mécaniques de bataille, siège et guerre navale (Codex « Le jeu »)

Conception : `docs/design/2026-09-25-bulles-partout.md` (lot B3). Suivi général : `docs/wip/bulles-partout.md`.

## Périmètre
Fiches `category: "mecanique"` (ids `cdx_jeu_…`) sur la bataille 3D, l'auto-résolution, les sièges
(campagne et assaut 3D), les incendies, les engins et la guerre navale ; une fiche par ordre du chef
(`entity: "order_…"`) ; `gameplay` ajouté à `cdx_deroute_debandade` ; liens `[[cdx_…]]` dans les
descriptions de `data/battle_orders/`.

Chiffres : tirés de `core/crates/sim-battle/src/` (sim.rs, impact.rs, unit.rs, site.rs, siege*.rs,
orders.rs, naval/), `core/crates/sim-campaign/src/` (battle_auto.rs, siege.rs, naval.rs, movement.rs,
battle_request.rs, battle_forecast.rs, dynasty.rs), `data/rules/*.json`, `data/naval/rules.json`,
`data/settlements/rules.json` (repli).

## État : terminé (32 fiches + gameplay de cdx_deroute_debandade), validateur vert
- Bataille (15) : moral, fatigue, formations, deploiement, charge, pieux, melee, tir, flancs,
  terrain_bataille, relief_bataille, meteo_bataille, general, resolution_auto, pertes_prisonniers.
- Siège (8) : siege, murailles, assaut, belier, sortie, engins_siege, huile_bouillante, incendies.
- Naval (4) : bataille_navale, abordage, brulots, maitrise_mer. Le débarquement est laissé à
  `cdx_jeu_traversee` (B2), qui le traitait déjà (fiche B3 supprimée).
- Ordres (5) : pied_a_terre, pavois, pas_de_quartier, ralliement, cri_de_guerre.

## Coordination avec B2/B4/B5
- `data/codex/_b3_links.md` : ids des autres lots liés depuis B3 (existants sur leurs branches,
  `cdx_galere` supposé). À supprimer après fusion de tous les lots.
- Alias cédés pour éviter les doublons : « schiltron » (B5 cdx_schiltron), « pavescheur » (B5
  cdx_pavois), « pierrières » (B5 cdx_mangonneau), « mâchicoulis », « hourds », « chemin de ronde »
  (B4 cdx_enceinte_de_pierre). Recontrôler les alias à la fusion (lots encore en cours).

## Points ouverts
- La barre d'ordres (`leader_orders_bar.gd`) affiche la description brute : les liens `[[…]]`
  ajoutés aux descriptions d'ordres doivent passer par `CodexText.format` (lot B1).
- L'ordre « incendier » existe dans le cœur mais n'a pas de commande à l'écran (dit dans la fiche).
