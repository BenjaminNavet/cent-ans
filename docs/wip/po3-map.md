# PO3 — Carte de campagne (lumière, forêts, étiquettes)

Branche `feat/po3-map` (depuis `feat/po-polish`, qui y est fusionnée : PO2, AN1a). Plan :
`docs/superpowers/plans/2026-09-27-po-polish.md` § PO3. Références : bible DA § 12.6, ADR 0097.
**État : fait, prêt à fusionner** (l'orchestrateur fusionne).

## Fait
- [x] 1. Lumière — `data/fx/atmosphere.json`, `campaign.seasons.<saison>` (schéma étendu, clés
  soleil requises) : `sun_elevation` 18-28°, `sun_azimuth` 240-250° (ouest-sud-ouest),
  `sun_color` chaud, `sun_energy` ; brume `fog_color` bleutée + `fog_sun_scatter` (dorée côté
  soleil) ; `grade` de carte en S douce (contraste 1,06-1,08, lift bleuté, hautes lumières
  chaudes, saturation 0,72-0,88, `shadow_saturation` 0,45-0,5) appliqué après l'étalonnage de
  saison. `CampaignAtmosphere.resolve_preset(season)` ; plus aucune valeur de soleil dans le code
  (les `@export` 38°/315° sont supprimés). Le soleil est reposé après le chargement (le relief ZG8
  `_apply_sun_elevation` impose 34° pendant la construction du terrain) et au changement de
  saison, jamais pendant le soir doré de `TurnLight`.
- [x] 2. Forêts — `foliage.gdshaderinc` (carte seulement) : 3 tons par bruit basse fréquence
  (`tone_period` 22 unités) + jitter par arbre ; échelle ±25 % par graine ; lisières : sur la
  rampe de la couverture forestière (splat réduit à 1024 px, `VegetationMask.forest_cover_image`,
  lié par `Vegetation._bind_forest_cover`), arbres à 62 % de hauteur et 45 % retirés ; arbres des
  champs et cœur de massif inchangés ; haies exclues. Aucun nouvel asset, semis natif (Rust)
  inchangé.
- [x] 3. Étiquettes — `settlement_layer.gd` : EB Garamond variable (graisse 700 / 600 / 500),
  tailles `UiType` (PO2 fusionné : Heading 20 cité, Body 17 ville, Caption 14 bourg, château,
  abbaye, village), encre plus sombre, halo de parchemin fin (contour 4 px, opacité 0,6) au lieu
  du contour opaque de 7 px (« pastille »). Dé-encombrement DA7d inchangé.
- [x] 4. `po_grade_test.gd` partie campagne active (C4) ; partie bataille laissée à PO4.
- [x] 5. Captures `game/tests/po3_shot.gd` (`--season=a,b`, `--out=`) → `docs/img/po/po3/`
  (été : large 1250, régionale 491, rapprochée 150 autour de Paris).

## Vérifications
- `po_grade_test.gd` OK ; `smoke.gd` OK ; `da7d_overlap_test.gd` OK (0 chevauchement) ;
  `settlements_render_test.gd` OK ; `c5_settlements_ui_test.gd` OK ; `tools/tests/test_atmosphere_schema.py` OK.
- `sz4b_colonies_forests_test.gd` : 6 échecs **déjà présents sur `feat/po-polish`** (maquette,
  moulin, fumées, forêt dense), identiques avec et sans PO3.
- Saturation DA7b (`scene_saturation measure`, 1280×720) — toutes ≤ 35 % :

| Vue | printemps | été | automne | hiver | base (été, avant PO3) |
|---|---|---|---|---|---|
| large (1250) | 32,7 | 34,3 | 34,0 | 23,1 | 36,4 |
| régionale (491) | 27,4 | 29,0 | 31,9 | 19,1 | 32,0 |
| rapprochée (150) | 31,8 | 34,1 | 34,9 | 15,2 | 41,2 |

## Banc PB1 (M4 Pro, fenêtre 1440×900, machine partagée : charge 18-130 pendant la mesure)
Metal plafonne à 6,9 ms (144 Hz) puis a sauté à 25-32 ms pour **base et PO3 à l'identique**
(contention GPU d'autres sessions) : inexploitable. Vulkan, temps GPU (ms), 2 paires alternées
base / PO3 dans le même créneau, moyennes :

| Vue | base | PO3 | écart |
|---|---|---|---|
| 1250 | 19,5 | 19,9 | +2 % |
| 491 | 26,0 | 26,3 | +1 % |
| 150 | 31,6 | 33,3 | +5 % |
| 40 | 34,4 | 34,1 | −1 % |
| chargement carte | 6660 ms | 6835 ms | +2,6 % |

Bruit entre deux passes identiques : jusqu'à ±50 % (150 : 16 → 32 ms sur la base). La vue 150
est à la limite ; à refaire sur machine calme avant la fusion définitive. Coût attendu : un
échantillon de texture et ~4 hachages par sommet d'arbre (carte seulement).

## Écarts / points ouverts
- Saturation obtenue surtout par l'étalonnage de carte (0,72-0,88) : la lumière chaude et la brume
  bleutée ajoutaient ~4-5 points. Si le joueur trouve la carte trop terne, relever la saturation
  des tons clairs plutôt que la saturation globale.
- La mer sombre pèse dans la mesure (saturation HSV forte des tons sombres) ; l'outil DA7b
  recadre pour la bataille, pas pour la carte.
- Banc PB1 à reconfirmer sur machine calme (voir ci-dessus).

## Prochaine étape
Fusion par l'orchestrateur dans `feat/po-polish`.
