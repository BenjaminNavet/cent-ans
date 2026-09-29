# OM I1 — intégration finale de la carte Oural–Méditerranée

Branche `feat/om-om2`, worktree `../gp-om-om2`. Profil cargo `i1`.

## Plan
1. Données D : provinces à 2 colonies, set_emba, colonies isolées sans port, autres incohérences.
2. Régénération géo complète (ordre de `om-om2.md`), commit dédié aux artefacts.
3. Points fins (fleuves, routes) : décalage `root_origin_tiles` × 256 dans le moteur.
4. Tests : cargo workspace, pytest, build.sh, Godot (smoke + tests carte).
5. Mesures : chargement, mémoire, tour d'IA.

## État
- [x] 1 (premier passage) : voir « Corrections de données ».
- [x] 2 régénération complète (build 5 min 22, splat 2 min 18, relief-shade 1 min 02, navgrid 6 s,
  rivers-render 14 s, horizon 25 s ; ≈ 9 min 30) : 443 provinces, 1 composante connexe, plus
  aucune île sans port ; commit `data: OM I1 regenerated geo artifacts`. Aucun fichier > 50 Mo.
  `geo settlements` + `navgrid` relancés après Pantelleria (port).
- [x] 3 : `FineGeoStore` lit `root_origin_tiles` de `relief_pyramid.json` ; index et points CAFV
  en coordonnées monde (+20 tuiles E2, +1280 unités en y), chemin de fichier en coordonnées de
  cache (`CafvTile.parse(bytes, offset_tiles)`). Test : `zg5b_fine_geo_test.gd` § 2b ; test réel
  de Rouen décalé. Aucun code Rust ne lit les tuiles CAFV (rendu seul).
- [x] 4 : cargo test --workspace 1 223 ok, 0 échec, 55 ignorés ; clippy -D warnings et fmt OK ;
  pytest 1 246 ok, 2 sautés ; import, smoke.gd (toutes étapes OK) et 22 tests Godot carte OK.
- [x] 5 : voir « Mesures ».

## Corrections de données
- 3 colonies minimum : Kubadabad (palais seldjoukide, Beyşehir), Göksun (Elbistan), Urzędów
  (Lublin), Târgoviște (Munténie, mention 1396), Floci (Munténie orientale, XVe s., incertain),
  Slatina (Olténie, 1368), Loukoml (Polotsk, 1078), Elmalı (Teke), Droutsk (Vitebsk, 1092).
- Hors carte (bord sud ≈ 29,3° N vers 20° E) : oasis de Marada, Awjila, Jalu (Ajdabiya) et Zella
  (Syrte) retirées ; remplacées par Marsa el-Brega et Zouétina (mouillages, ports, incertains) et
  Madinat Sultan (Surt médiévale).
- set_emba (53,5° E, à l'est du bord) remplacée par Jaïk (site de la Horde d'Or près d'Ouralsk).
- Valaam, Djerba, Gozo, Poide (Ösel) : `port: true` (îles sans autre accès) ; Altefähr (bac de
  Rügen, port) ajoutée : Bergen de Rügen n'était plus isolée sans port.
- Colonies à plus de 40 km de leur province (Voronoï) : Borgå → Tavastie, Goslar et Hildesheim →
  Brunswick, Pantelleria → province de Tunis avec `owner: fac_sicily` (îlot rattaché à la côte
  africaine par la règle des îlots), Brzeg → Haute-Silésie avec `owner: fac_legnica` (enclave :
  le duché de Wrocław sépare Legnica et Brzeg) + Chojnów ajoutée à Legnica ; Luna : poids
  Voronoï 0,4 → 0,7 (Illueca).
- Restent ramenées de loin (> 40 km), à revoir : Volok Lamski (Torjok, enclave novgorodienne
  au-delà de Tver), Gorokhovets (Vladimir), Kamianiets (Hrodna, en fait terre de Brest ; déplacer
  demande une 3e localité attestée pour Hrodna).

## Bug trouvé : dépressions sous le niveau de la mer
La terre ≤ 0 m est de l'eau pour le jeu (plan d'eau Y = 0, `relief_shade.upsample_sign`,
navgrid) : toute la dépression caspienne (−28 m, delta de la Volga, Saraïtchik), la Qattara, les
chotts, le Jourdain étaient de la mer. `terrain.lift_inland_depressions` (appelé par
`geo build`) remonte à 0,5-1 m la terre ≤ 0 m non reliée à l'océan (graine : golfe de Gascogne) ;
les basses terres reliées à la mer (Pays-Bas) gardent le comportement d'avant. Test
`tools/tests/test_inland_depressions.py`.

## Tests Rust corrigés (données D, pas les règles)
- eq2_balance : 57 bâtiments de colonies D hors des règles (cathédrale en bourg → collégiale,
  comptoir sans foire → foire à la place du marché, université sans collégiale → collégiale
  ajoutée, marché/port retirés des villages et châteaux).
- dp2_explain : `neutral()` = première faction en paix **de la foi de la France** (la première
  par id était l'Alanie, dont les objections de foi masquaient le point bloquant).
- feudal_titles::no_objective_victory_at_start : 31 titres D avaient tous leurs objectifs déjà
  remplis ; un objectif historique non atteint ajouté à chacun (Chios, Smyrne, Samogitie, Caffa,
  Halych…), au plus 3 objectifs (schéma).
- m10_events::black_death : la vague part du sud, qui est maintenant l'Égypte : Le Caire au
  premier pas (touché à l'automne 1347), plus Malaga.
- f7_events passe (seed inchangée).

## Godot
- `core/build.sh` en profil debug avec la cible partagée compile godot-bridge contre des
  dépendances d'un autre checkout (erreurs `set_pyramid`, `HarbouredFelon`) : dylib construite
  en profil `i1` et copiée à la main dans `game/bin/libcent_ans.debug.dylib`.
- import OK ; smoke.gd OK (7 min 20 en profil non optimisé).
- Tests carte : om1_wide_world, om3_terrain, zg2, zg4, zg5b, zg6, zg7a, zg7b, zg7c, zg8, pb3g,
  po5, rs_k (2), sz6, da7d, ga1, ga2, ga4, ga5, mf1, c5_settlements_ui : OK. Corrigés : zg2, zg8
  (index de morceau `% 16` → `chunks_x`), zg4 (256 morceaux → `chunk_count()`), zg8 (sonde des
  Pyrénées oubliée par la migration +1280).

## Mesures (texte, machine partagée)
- Chargement de la carte de campagne réelle (headless, `campaign_map.tscn`, script de sonde
  hors dépôt) : OM 11,4 s (`startup.total_ms` 9 310 : relief_landcover 7,8 s, terrain 2,3 s,
  décor 2,1 s, 672 morceaux) contre main 6,7 s (4 700 ; 256 morceaux). Mémoire : RSS max
  2,89 Go contre 1,61 Go ; mémoire statique Godot 2,08 Go contre 1,10 Go (≈ ×1,8 pour ×2,6 de
  surface). VRAM non mesurable en headless (estimation OM1 : 1,38 Go de textures de base).
- Tour d'IA (`turn_perf 10 1 1`, profil release sans debug) : main 892 tours de faction,
  moyenne 2,53 ms (p99 13,9, max 25) ; OM avant correctif 1 746 tours, 8,50 ms (p99 35,
  max 117), 33,7 s ; après correctif 6,14 ms (médiane 3,71, p99 30, max 89 fac_empire), 25,5 s
  (main 6,1 s). Par tour de jeu ≈ 1,07 s de planification IA (177 factions) contre 0,23 s.
- Profil (`sample`) : 61 % du fil principal dans `city_state`, dont `are_neighbors` (O(P) par
  paire de factions, appelé pour toutes les factions par l'émissaire IA, le commerce, le
  changement d'allégeance). Correctif : `CampaignState::neighbour_factions` (une passe) aux
  trois boucles, `are_neighbors` lit la cité sans relire la province. Test
  `om_i1_neighbours.rs` (égalité avec `are_neighbors`). Reste : grille (`GridPlanner`),
  `agent_dijkstra`, `attitude`, `faction_power` (O(armées + colonies) par appel).

## Prochaine étape
Lot terminé. Points ouverts : Volok Lamski, Gorokhovets, Kamianiets ramenées de loin ;
268 colonies ramenées dans leur province (la plupart côtières, < 20 km) ; objectifs ajoutés aux
31 titres D à relire par un historien ; coût restant du tour d'IA (×4,6 par tour de jeu contre
main) ; chargement (relief_landcover 7,8 s).

## Vérification après fusion de feat/om (P2 + main : FE8, Q6, DZ ; b9e8a069b)
- `git merge --ff-only feat/om` ; dylib i1 reconstruite ; import OK.
- cargo test --workspace : 1 223 ok, 0 échec, 55 ignorés ; clippy, fmt OK ; pytest 1 246 ok,
  2 sautés. Aucun test FE8 à recaler.
- Godot : 22 tests carte + dz_diplo_borders, p2d_ui, q6_diplomacy, q6_side_panel, q6_toasts,
  q6_ui : OK. q6_diplomacy / q6_ui impriment des `SCRIPT ERROR` (SimFacade introuvable à la
  compilation) identiques sur main : préexistant. `q3_playtest.gd` est un pilote en fenêtre (pas
  un test headless) : bloqué en headless, arrêté après 32 min ; il a réécrit
  `user://settings.cfg` (fenêtré, 1920 × 1080, conseiller et voix réarmés).
- century_probe 50 tours, graines 1-5 (1 min 42) : banqueroutes toutes factions 5,25 / fac. / déc.
  [5,12-5,50] (FE8 50 t. avant F8 : 7,22 ; 464 t. après F8 : 3,21) ; petites factions 6,03-6,47
  (réf. 50 t. avant F8 : 7,90-9,12) ; 28 factions d'avant FE 0,03-0,23 (464 t. après F8 : 0,12) ;
  révoltes 15,2 / 200 t. [4-36] (464 t. après F8 : 8,7) ; commise de Guyenne 5/5 au tour 1 ;
  guerre FR-EN 87 % [76-98] sur 50 t. ; recettes France t ≤ 50 : 1 713 089 (après F8 : 1 712 514).
