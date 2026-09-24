# CV1 — Campagne vivante (partie 1)

Branche : `worktree-agent-ae5263563bc920e75` (partie de `main` `93d466c`). Rendu seulement :
aucune règle, l'information vient du pont (`get_date_label`, `get_turn`, `get_province_state`,
`settlement_detail`).

## État : terminé (à fusionner par l'orchestrateur)

## Architecture

- `game/scripts/map/campaign_life.gd` (`CampaignLife`) : nœud unique branché dans `campaign_map.gd`
  (création après les colonies, `refresh(sim)` dans `refresh_all`, `update_view(distance)` dans
  `_process`, `invalidate()` au chargement). Relit population, dévastation, sièges et bâtiments une
  fois par tour. Options : `--season=spring|summer|autumn|winter`, `--devastate=<province>:<0-100>[,…]`,
  `--no-life` (A/B), `--life-off=terrain,smoke,mills,ambient` (coût par partie).
- `season_visuals.gd` : saison du libellé de date → paramètre de shader global `campaign_season`
  (`project.godot` § `[shader_globals]`, poids printemps/été/automne/hiver), transition 2,5 s.
- `game/shaders/campaign_life.gdshaderinc` : toutes les fonctions (saisons, neige, terroirs, brûlis,
  ombres de nuages), incluses par `terrain.gdshader` ; crochets d'une ligne marqués `// CV1`.
  `foliage.gdshader` : fonction `season_foliage` + global (feuillus roux/nus, résineux gardés).
- `terroir_mask.gd` : texture 1024² (R champs, G vigne, B brûlis, A pâtures) autour des colonies
  et hameaux selon population et dévastation (≈ 30 ms, reconstruite si l'état change).
- `settlement_growth.gd` : niveau visuel 0 village, 1 bourg (ouvert, halle), 2 ville (murailles),
  3 cité (cathédrale, halle, donjon, faubourgs) ; château Kenney fusionné en un maillage si
  fortification ≥ 6 ; Paris exclu (monument L1). Remplace la maquette par
  `SettlementLayer.replace_model` (accesseurs CV1 ajoutés en fin de `settlement_layer.gd`).
- `life_effects.gd` : fumées de cheminée (≈ 3 100 panaches, `life_smoke.gdshader`), fumées
  d'incendie (hameaux brûlés, villages en ruine, faubourgs assiégés), moulins à vent (ailes qui
  tournent, `life_windmill.gdshader`, arrêtées en pays dévasté), surcouche `life_overlay.gdshader`
  (neige sur les toits l'hiver, suie des ruines).
- `life_ambient.gd` : vols d'oiseaux (`life_birds.gdshader`), bateaux sur les fleuves Strahler ≥ 5,
  navires entre ports voisins par la mer.
- Modèles : `tools/blender_scripts/settlements_cv1.py` (séparé de `settlements.py`, que BR1
  réécrit) → `game/assets/models/settlements/{bourg,cite}_{a,b}.glb`, `windmill_{body,sails}.glb`.
- Test : `godot --headless --path game --script res://tests/cv1_campaign_life_test.gd`.

## État des lots

- [x] 0. Squelette
- [x] 1. Saisons visibles (terrain, feuillage, neige d'altitude et de plaine selon nord/est)
- [x] 2. Terroirs et fumées de cheminée
- [x] 3. Colonies qui grandissent
- [x] 4. Dévastation visible
- [x] 5. Vie ambiante (nuages, oiseaux, bateaux, navires, moulins)
- [x] Mesures FPS, captures `docs/audit/captures/cv1/` (avant = `--no-life`), smoke

## Mesures (M4 Pro, 1600 × 900, `--fps-probe`, machine très chargée : charge 25 à 126)

A/B dans la même session (`--no-life` = rendu de `main`), moyennes de 3 paires alternées :

| Vue | CV1 | sans CV1 | Écart | Appels de rendu |
|---|---|---|---|---|
| Beaune proche (d = 45), printemps | 47,6 | 47,9 | −1 % | 922 / 861 |
| Paris moyen (d = 400) | 54,0 | 57,7 | −6 % | ≈ 640 / 600 |
| France (d = 1 200) | 67,3 (2 essais) | 59,9 | bruit | 465 / 465 |
| Beaune proche, hiver | 36-51 | 45 | bruit | 1 101 / 861 |

Écarts dans le bruit de mesure (un même essai varie de 30 à 55 i/s). L'hiver ajoute une passe
de surcouche par maquette de colonie visible (neige des toits : ≈ +240 appels de rendu au palier
proche) ; les hameaux n'en ont pas.

## Points ouverts

- Surcouche de neige de l'hiver : +25 % d'appels de rendu au palier proche (une passe par surface
  de maquette). Si c'est trop, la mettre dans les matériaux des maquettes (lot BR1) plutôt qu'en
  `material_overlay`.
- Relecture des colonies (`settlement_detail` × 374) : ≈ 150 ms en debug, une fois par tour.
- Bateaux posés sur `surface_height_at` : si V4 creuse le lit des fleuves, les baisser d'autant.
- Château Kenney : toits bleus du kit (teinte grise multipliée) ; à remplacer si BR1 fournit un
  château réaliste.
- Paris exclu par identifiant (`SettlementGrowth.EXCLUDED`) : pas de point d'accroche L1 dans `main`.
- Neige de plaine : bruit de valeur légèrement en plaques au zoom moyen.

## Prochaine étape

Rien : fusion par l'orchestrateur.
