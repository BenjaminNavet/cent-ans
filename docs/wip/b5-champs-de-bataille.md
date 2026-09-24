# Lot B5 — Champs de bataille tirés de la campagne

Plan : `docs/design/2026-09-24-rapprochement-total-war.md`. Branche du worktree B5 (non fusionnée).

## Constat de départ
- Le cœur tirait déjà relief / forêts / boue / rivière selon le terrain de la province (7 terrains) et
  la météo selon la saison ; mais ni côte, ni village, ni haies, ni mares, ni sol de saison (neige au
  sol sans chute de neige, boue sans pluie). Le rendu ignorait le terrain : même horizon, même densité
  de bois partout.

## État : terminé (non fusionné)
- [x] Cœur : `core/crates/sim-battle/src/site.rs` (sol de saison, côte sur un flanc, mares de marais,
  village ou ferme avec maisons, haies/clôtures des courtils, haies du bocage, fossés du marais), tiré
  d'un flux dérivé de la graine sans la consommer (`BattleRng::derive`) : les batailles d'avant B5
  gardent relief, forêts, rivière et tous les tirages suivants (test dédié).
- [x] Effets : haie = couvert contre les traits (×0,6 si le tir traverse une haie à moins de 14 m de la
  cible), brise la charge de cavalerie (pas d'impact, −5 moral, journal) ; fossé = brise la charge ;
  haie/clôture/fossé ralentissent le franchissement (cheval ×0,3/0,6/0,4) ; village = couvert ×0,6,
  ruelles lentes (0,8 pied / 0,55 cheval), charge brisée ; plage ×0,85 ; mares = eau peu profonde ;
  neige au sol ×0,9 sans chute de neige.
- [x] `BattleSetup.coastal` (province côtière, rempli par la campagne) et `BattleSetup.village`
  (forcé / interdit / tiré), défauts serde ; sièges : ni côte ni village.
- [x] Pont : `get_terrain()` exporte `terrain, season, ground, ground_label, woodland, pools, obstacles,
  coast?, village?`.
- [x] Tests `core/crates/sim-battle/tests/site.rs` (11 : un champ par terrain, déterminisme,
  compatibilité des tirages, sol de saison, côte, village, couvert de la haie, charge brisée, ralentis).
- [x] Rendu : `battle_village.gd` (maisons chaume/colombage/grange/église en boîtes regroupées par
  `BattleSiegeBatcher`, clôtures de plessis fusionnées, mares, roselières par tuiles, mer
  `battle_sea.gdshader`) ; `battle_terrain.gd` (biomes : relief et bois de l'horizon, densité des bois,
  rochers ; feuillage de saison ; haies semées en buissons + chênes têtards ; splat : fossés, ruelles,
  mares, plage ; sol enneigé/détrempé ; neiges des sommets via `snow_line` dans `battle_ground`) ;
  `battle_vegetation.gd` (herbe de saison, neige au sol).
- [x] Options : `--terrain= --season= --village --no-village --coast --river` (mise en place),
  `--ground=` (rendu), `--no-site` (A/B perf). Accroche d'une ligne dans `battle_scene.gd`.
- [x] Captures `docs/img/b5/` : `b5_plaine_village`, `b5_colline_boisee`, `b5_marais`, `b5_hiver_cote`.
- [x] Smoke vert (21 « smoke OK »), cargo fmt/clippy/test verts.

## Perf (M4 Pro, `--disable-vsync`, 600 images, même binaire, A/B `--no-site`, passes alternées)
Les deux variantes plafonnent à 60 i/s (cadence imposée par l'écran malgré `--disable-vsync`) : écart
de FPS non mesurable ici. Coût géométrique :
| Config | `--no-site` | B5 |
|---|---|---|
| bocage + village + côte, 1160 soldats | 1,24 M prim. / 496 appels | 1,29 M / 536 |
| idem, `--units=20` (4800 soldats) | 1,89 M / 640 | 1,93 M / 682 |
| marais (gros plan roseaux) | 1,21 M / 659 | 1,04 M / 588 (bois clairsemés du biome) |

## Points ouverts
- L'IA tactique ne cherche pas encore les haies ni le village (les archers anglais s'y placeraient) :
  à ajouter dans `ai.rs` (choix de position défensive).
- Minicarte et dialogue d'avant-bataille n'affichent pas village/côte/sol (`ground_label` exporté).
- Arbres d'hiver : feuillage bruni (pas d'arbres nus, faute de maillage dédié).
- Mesure de FPS au-delà de 60 i/s à refaire sur une session sans plafond d'affichage.
