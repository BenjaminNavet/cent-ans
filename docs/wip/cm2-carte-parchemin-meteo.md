# CM2 — carte de campagne : vue stratégique parchemin, météo, lumière de fin de tour

Branche `worktree-agent-a7241565332549af6` (partie de `main` `ad08ea16`).

## Plan (un commit par lot)
1. Vue stratégique parchemin (fondu selon la distance caméra, zoom maximal).
2. Météo de campagne : `core/` (déterministe : graine, date, province), pont, rendu (pluie, neige,
   brouillard matinal, orages), `campaign_ambience.gd` lit la nouvelle source.
3. Lumière dorée pendant le tour de l'IA (si le temps le permet).

## État
- [x] 0. Squelette
- [x] 1. Parchemin : `strategic_view.gd` (poids 1080 → 1440, global `campaign_parchment`),
  `parchment_common/map/sea.gdshaderinc` (crochets `// CM2` dans terrain, water, river_water),
  `terrain_parchment.gdshader` (substitué au poids 1 : le rendu 3D n'est plus payé),
  `parchment_overlay.gd` (noms, vignettes pré-rendues, jetons, navires et monstres),
  `parchment_decor.gd` (roses, navires, monstres placés par la distance à la côte)
- [x] 2. Météo : cœur `sim-campaign/src/weather.rs` (fonction pure : graine, tour, saison,
  capitale ; fronts classés du sec à l'humide, chances par climat `data/rules/campaign_weather.json`),
  pont `campaign_sim_weather.rs` (`get_campaign_weather`, `get_province_weather`), ADR 0027
  (visuelle seulement). Rendu : `campaign_weather_view.gd` (masque par province, nuées
  `campaign_clouds.gdshader`, pluie / neige en particules de près, éclairs), sol
  `campaign_weather.gdshaderinc` (mouillé, neige fraîche, brouillard matinal qui se lève,
  ombres des nuées), nuées dessinées sur le parchemin. AU1 : `CampaignAmbience.map_weather`.
  Test `game/tests/cm2_parchment_weather_test.gd`, `cargo test --test cm2_weather`.
- [x] 3. Lumière de fin de tour : `turn_light.gd` (soleil bas à l'ouest, doré, −18 % d'énergie au
  plus, tant que le bandeau « Tour des autres factions » est affiché ; aube rosée au retour).
  `--dusk` fige le soir (capture `apres_soir_ia_400.png`).
- [x] Mesures FPS (3 zooms), captures `docs/audit/captures/cm2/`
- [x] Fusion de main (44bcdc16, sans conflit), cargo fmt/clippy/test, build.sh, import, smoke (24 OK), test CM2 OK

## Mesures de référence (avant CM2, Vulkan, 1600×900, `--fps-probe`, charge ≈ 30)
| Distance | i/s | GPU ms |
|---|---|---|
| 400 | 50,1 | 19,94 |
| 1000 | 98,5 | 6,38 |
| 1500 | 65,4 | 10,28 |

## Mesures après CM2 (Vulkan, 1600×900, `--fps-probe`, A/B dans la même session, charge 11-26)
« avant » = `--no-parchment --no-map-weather` ; météo réelle du tour 1 (nuées à 1000).
| Distance | GPU ms après (2 passes) | GPU ms avant (2 passes) | écart |
|---|---|---|---|
| 400 | 24,10 · 25,62 | 25,59 · 25,46 | −3 % (bruit) |
| 1000 | 12,31 · 11,85 | 12,30 · 11,20 | +3 % |
| 1500 (parchemin) | 11,96 · 12,08 | 12,24 · 12,40 | −3 % |
| 1250 (fondu, avant réglage) | 13,64 · 14,25 | 11,81 · 9,40 | +15 à +50 % |
À 1500, le terrain passe au shader « parchemin seul » (`terrain_parchment.gdshader`) : moins
cher que le rendu 3D. Dans la bande de fondu, les deux rendus sont calculés : bande resserrée à
1180 → 1440 (transitoire pendant le zoom). Couche 2D : ≈ 3 ms CPU (vignettes pré-rendues ;
`draw_colored_polygon` par image coûtait 70 ms pour 130 villes).

## État : terminé (à fusionner par l'orchestrateur)

## Points ouverts
- Dans la bande de fondu (1180 → 1440), terrain 3D et parchemin sont calculés tous les deux
  (+15 à +50 % GPU tant que la caméra y reste) ; au-delà, shader « parchemin seul ».
- Météo sur la mer : aucune (le masque est par province terrestre) ; nuées et pluie s'arrêtent
  à la côte.
- Les nuées (non éclairées) ne prennent pas la teinte dorée du soir.
- La météo n'a aucun effet de règle (ADR 0027) ; les batailles gardent leur tirage N1.
- Jetons d'armée : la sélection passe toujours par les marqueurs 3D (masqués mais cliquables aux
  mêmes positions) ; pas d'animation de marche des jetons en dehors de celle de M4.
- Vignettes de villes : une seule forme (pré-rendue) ; les capitales ont un fanion aux couleurs.
- Toute la couche 2D est redessinée à chaque image en vue parchemin (≈ 3 ms CPU).

## Prochaine étape
Rien : fusion par l'orchestrateur.
