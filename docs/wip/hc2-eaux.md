# HC2 — eaux lisibles sur la carte de campagne (lacs, étangs, mares)

Worktree `../gp-hc2`, branche `feat/hc2` (issue de `feat/hc`). ADR 0161 §3. Rendu seul, aucune
règle ni cuisson autre que `geo lakes`. **En pause (demande du joueur, 02/10 21 h).**

## État
- [x] Plus de lacs : `min_area_px` 30 → 8 (défaut de `LakesParams` et de la CLI), `lakes.json`
      762 → 1231 lacs (13 473 → 17 046 sommets). Le seuil 4 donne exactement le même fichier (un
      bassin exige un cœur plein, `flat_basins`). Noms : couche Natural Earth absente du worktree,
      lue dans le checkout principal (`--natural-earth
      ../game_project/tools/geo/raw/natural_earth/ne_10m_lakes/ne_10m_lakes.shp`), 67 lacs nommés
      comme avant. `LakesRenderer` : 14 813 triangles, construction 43-51 ms, 0 échec.
- [x] Lacs éclaircis (terrain.gdshader, bloc des lacs) : `lake_color` (0.07, 0.23, 0.38),
      `lake_shallow_color` (0.12, 0.28, 0.36), `lake_bank_color` (0.15, 0.14, 0.085),
      `lake_sky_color` (0.38, 0.55, 0.72), `lake_sky_reflect` 0,35. Nappe : `LakesRenderer`
      `deep_color` (0.29, 0.52, 0.65), `shallow_color` (0.38, 0.57, 0.64), `sky_reflect` 0,8.
      Mesure (cœur des nappes, sRGB) : eau ≈ (100, 133, 142) contre sol ≈ (87, 99, 59) ; hiver
      eau ≈ (102, 130, 152) contre sol ≈ (115-138, gris neige).
- [x] Étangs et mares généralisés (`relief_landcover.gdshaderinc`) : cellules doublées par
      paliers d'empreinte, rangées décalées, densité lue sur l'emprise de l'étang, berge, couverture
      moyenne au-delà du dernier palier. Uniformes : `rl_water_ref_height` 1080, `rl_pond_min_px`
      5, `rl_pond_max_cell_px` 22,4, `rl_pond_fill` 0,8, `rl_pond_bank` 0,05, `rl_pond_bank_amount`
      0,75, `rl_pond_far_cover` 0,35, `rl_pool_min_px` 8, `rl_pool_amount` 0,45, `rl_pool_opacity`
      0,85, `rl_pool_max_scale` 16, `rl_pool_lake_tint` 0,85. `rl_surface` reçoit `VIEWPORT_SIZE.y`.
      Dombes (image 640×400) : 25 nappes d'étang ≥ 6 px à rig 150, 6 à rig 300.
- [x] Test `hc_water_test.gd` : headless OK ; avec fenêtre, contrôle chiffré de la Dombes OK
      (réglages finaux). Outils `hc_water_view.gd`, planche `hc_water_shots.gd`.
- [x] Planches finales : `~/.cache/cent_ans/hc/hc2_water.jpg` et `hc2_water_winter.jpg`.
      Lectures de planche : 3 / 3 (budget épuisé ; la planche d'hiver n'a pas été lue).

## Tests
- Passés sur l'état final : `hc_water_test.gd` avec fenêtre (OK), `tools/tests/test_lakes.py`
  (10 réussis), ruff.
- Passés sur un état intermédiaire du shader (avant les derniers réglages de valeurs et le
  décalage des rangées) : `smoke.gd` (exit 0), `ss_lakes_test.gd` OK, `hc_water_test.gd` headless OK.
- **Pas repassés sur l'état final (interrompus par la pause)** : `smoke.gd`, `ss_lakes_test.gd`,
  `hc_water_test.gd` headless, `tb1_seasons_test.gd`.

## Prochaine étape
Relancer `godot --headless --path game --script res://tests/smoke.gd`, `ss_lakes_test.gd`,
`hc_water_test.gd`, `tb1_seasons_test.gd` ; puis rapport final et relecture HC3.

## Points ouverts
- Écart à la spec : tailles d'étang en pixels d'un écran de 1080 de haut (même cadrage à toute
  résolution), moyenne géométrique des deux axes écran ; minimum 5 px (≈ 5-10 selon le palier) au
  lieu de 6 px écran bruts. `rl_water_ref_height = 0` rend les pixels bruts.
- Part d'eau mesurée moitié moindre en hiver : probablement les lacs lointains voilés par la brume
  d'hiver (critère magenta), à confirmer sur la planche d'hiver.
- Un passage du balayage `--views` avec rig 60 en première vue a donné une échelle incohérente
  (2,63 × 0,63 px au lieu de 5,69 × 2,94) et un autre a tourné > 20 min sous forte charge machine ;
  non reproduit ensuite. Cadrage à vérifier (`hc_water_view.gd:frame`).
- Marais des Fens : masse sombre de la roselière (`rl_reed_color`, non modifié), mares visibles
  mais discrètes.
- Lacs historiques absents des données (lot ultérieur, touche `land_mask.png` donc les règles) :
  Grand-Lieu et Loch Ness (eau dans le masque, sans lac : sous le niveau minimal ou non plat),
  étang de Berre (relié à la mer), Windermere, Paladru, Aiguebelette, Joux, Nantua, Saint-Point,
  Léon/Soustons, Haarlemmermeer, Whittlesey Mere (absents du masque). Zurich : lac présent, centre
  hors eau. Lacs de 2-4 km² (60 composantes de 4-7 px) écartés par le critère de cœur plein.
- `game/tests/gc_maquettes_test.gd.uid` non suivi : n'appartient pas à HC2, laissé tel quel.
