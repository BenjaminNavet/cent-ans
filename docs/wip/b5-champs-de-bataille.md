# Lot B5 — Champs de bataille tirés de la campagne

Plan : `docs/design/2026-09-24-rapprochement-total-war.md`. Branche du worktree B5 (non fusionnée).

## Constat de départ
- Le cœur tirait déjà relief / forêts / boue / rivière selon le terrain de la province (7 terrains) et
  la météo selon la saison ; mais ni côte, ni village, ni haies, ni mares de marais, ni sol de saison
  (neige au sol sans chute de neige, boue sans pluie). Le rendu ignorait le terrain : même horizon,
  même densité de bois partout.

## État
- [x] Cœur : `core/crates/sim-battle/src/site.rs` (sol de saison, côte sur un flanc, mares de marais,
  village ou ferme avec maisons, haies/clôtures/fossés des courtils, haies du bocage, fossés du marais),
  tiré d'un flux dérivé de la graine sans la consommer (`BattleRng::derive`) : les batailles
  d'avant B5 gardent relief, forêts, rivière et tous les tirages suivants.
- [x] Effets : haie = couvert contre les traits (×0,6 derrière une haie traversée par le tir),
  brise la charge (pas d'impact, −5 moral) ; fossé = brise la charge ; clôture/haie/fossé ralentissent
  (cheval surtout) ; village = couvert (×0,6), ruelles lentes (0,8 pied / 0,55 cheval), charge brisée ;
  sable de la plage ×0,85 ; mares = eau peu profonde (comme un gué) ; neige au sol ×0,9 sans chute
  de neige.
- [x] `BattleSetup.coastal` (province côtière) et `BattleSetup.village` (forcé / interdit / tiré),
  défauts serde ; sièges : ni côte ni village.
- [x] Pont : `get_terrain()` exporte `terrain, season, ground, ground_label, woodland, pools,
  obstacles, coast?, village?`.
- [x] Tests `core/crates/sim-battle/tests/site.rs` (un champ par terrain, déterminisme, compatibilité
  des tirages, sol de saison, côte, village, couvert de la haie, charge brisée, ralentissements).
- [ ] Rendu Godot : maisons (chaume/colombage), haies, clôtures, fossés, mares et roseaux, neige/boue,
  densité de forêt, horizon par biome, mer.
- [ ] Options de capture `--terrain= --season= --village --coast` ; captures `docs/img/b5/`.
- [ ] Perf `--benchmark` (passes alternées avant/après).

## Prochaine étape
Rendu Godot (`battle_terrain.gd`, `battle_vegetation.gd`, nouveau `battle_village.gd`).
