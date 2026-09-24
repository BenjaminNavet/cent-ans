# V3 — atmosphère : lumière, ciels, feu et fumée (lots A1-05, A1-14, A1-13)

Agent V3, 25/09. Rendu seulement (aucune règle touchée). Source : `docs/audit/a1-visuel.md`.

## Plan
1. **A1-05** ciels HDRI + étalonnage (LUT 3D générée) par météo et saison, bataille et campagne.
   - HDRI Poly Haven CC0 2k dans `game/assets/third_party/skies/` (2 de D0 + 7 ajoutés).
   - `tools/cent_ans_tools/hdri_sun.py` : position du soleil dans chaque HDRI (aligne la lumière).
   - Réglages : `data/fx/atmosphere.json` (schéma `data/schemas/fx_atmosphere.schema.json`,
     test `tools/tests/test_atmosphere_schema.py`).
   - Code : `game/scripts/visual/atmosphere_library.gd` (ciels, LUT), `game/shaders/hdri_sky.gdshader`,
     `battle_atmosphere.gd` et `campaign_atmosphere.gd` branchés dessus.
2. **A1-14** qualité de rendu : `game/scripts/visual/render_quality.gd`, réglage `video/quality`
   (Basse, Moyenne, Haute, Ultra) dans Réglages > Affichage ; SSIL, brouillard volumétrique,
   ombres en cascades, glow, SDFGI (Ultra, bataille). `--quality=` en ligne de commande.
3. **A1-13** feu et fumée : flipbooks procéduraux (`tools/cent_ans_tools/fire_flipbooks.py` →
   `game/assets/textures/fx/`), shaders `fire_flame` / `fire_smoke` / `fire_ember`, lumière
   vacillante, dans `siege_fire_fx.gd` ; réglages `data/fx/siege_fire.json` (+ `embers`).

## État
- [x] Squelette, HDRI téléchargés, crédits
- [x] A1-05 : ciel HDRI aligné sur le soleil (plafond de luminance : sinon le soleil peint entre
  dans l'ambiance et les ombres disparaissent), LUT par saison puis météo, ambiance neutre
  partielle (le ciel HDRI bleuit trop les faces à l'ombre), soleil plafonné à 36°.
  Campagne : ciel + LUT de saison (suit le libellé de date ; `--season=` pour les captures).
- [x] A1-14 : `RenderQuality` + réglage ; brouillard volumétrique (densité par météo, nappe basse
  `FogVolume` par temps de brouillard).
- [x] A1-13 : flammes, fumée, braises, lumière vacillante (couleur et position).
- [ ] Mesures FPS / GPU (en cours), captures `apres_*`
- [ ] import + smoke (23 « smoke OK »)

## Mesures
Banc : `godot --path game --disable-vsync --resolution 1600x900 res://scenes/battle/battle.tscn --
--benchmark --bench-at=90 --units=20` (40 unités, ~4 590 soldats, la plus grosse bataille du banc).
Metal (pilote par défaut) plafonne à 60/120 images/s et ne rend aucun temps GPU : le temps GPU est
mesuré sous `--rendering-driver vulkan` (MoltenVK) et imprimé par le banc (`render … ms GPU`).
La machine est partagée (charge moyenne 30-45 pendant les mesures) : des exécutions séparées varient
du simple au double. D'où le banc **A/B dans le même processus** : `--bench-ab=legacy,high,…` alterne
les niveaux toutes les 30 images et donne la médiane GPU par niveau (même charge pour tous).
`legacy` reproduit les réglages d'avant V3 (SSAO ultra, ombres douces ultra, sans SSIL ni brume).

| Mesure (Vulkan, ms GPU, médiane) | legacy (avant) | Haute (défaut) | Moyenne | Ultra |
|---|---|---|---|---|
| A/B 4 niveaux (charge forte) | 21,66 | 18,46 (sans SSIL) | 16,46 | 28,80 |
| A/B legacy / Haute + SSIL | 23,58 / 18,35 / 21,41 | 21,92 / 13,72 / 18,59 (pluie : brume volumétrique active) | | |
| Siège `--siege --bench-at=120` | 15,89 | 15,94 | | |

Coûts isolés (A/B, même exécution) : SSIL basse qualité demi-résolution ≈ +1,5 ms ; brume
volumétrique permanente ≈ +1,8 ms (d'où « par mauvais temps » seulement en Haute) ; Ultra ≈ +33 %
(SDFGI, SSIL haute, brume permanente, MSAA 4×). Haute reste **moins chère** que l'ancien réglage
(-7 à -25 %) : l'économie vient de SSAO « high » au lieu d'« ultra » et du filtre d'ombres « high »,
qui paient SSIL et la brume.

Exécutions séparées, Metal, FPS moyens (plafonnés à 60) : avant 58,6 (médiane 60) ; voir plus bas
les mesures finales.

## Captures (`docs/audit/captures/v3/`, 1440×900 effectif)
`avant_*` (main) et `apres_*` : bataille clair / pluie / brouillard / neige d'hiver / automne / vue
haute (`--camera=600,400,22,35`), Basse et Ultra, bombarde (banc B4), incendie (proche, large,
ruines), campagne (proche, moyen, automne, hiver via `--season=`).

## Écarts et limites
- Ombres de nuages projetées (fiche A1-14) : non faites (la lumière directionnelle de Godot n'a pas
  de projecteur ; il faudrait toucher `battle_ground.gdshader` et `terrain.gdshader`).
- Maisons qui noircissent progressivement en brûlant : non fait (instances `MultiMesh` sans couleur).
- Ciel HDRI statique (pas de nuages qui défilent : le recalcul de la radiance à chaque image coûte).
- Campagne : la bascule de saison lit le libellé de date (`get_date_label`) toutes les secondes.
- SDFGI (Ultra) mesuré dans l'ensemble Ultra, pas isolé.
