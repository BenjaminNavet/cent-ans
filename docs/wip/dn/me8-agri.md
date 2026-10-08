# ME8 — paysages agricoles et saisons (branche dn/me8-agri)

## Fait
- `data/map/agri_landscapes.json` (+ schéma) : 6 paysages (openfields du nord, bocage de l'ouest, vignes en terrasses, oliveraies/vergers du sud, huertas/rizières, brûlis de l'est), 52 régions en ellipses lon/lat, `material_fallbacks` (rice_paddy, huerta_plots, fruit_orchard_rows, swidden_ash -> matières existantes), `seasonal_variants` (4 modèles du catalogue map_extra).
- `cent-ans geo agri-regions` -> `data/map/agri_regions.png` (L8, 1/4 de la grille ; valeur = ligne 8..15 de la table du sol).
- `HbGround` : table 18 x 16 (lignes 8+ = paysages, x=17 `open_to_farm`), masque `hb_agri`. Shader `hb_ground.gdshaderinc` : le paysage prend le pas sur le biome pour le parcellaire ; `hb_agri_splat` convertit une part de la prairie en cultures (terrain.gdshader, après `terroir_splat`).
- `AgriSeasons` : résout (classe d'arbre, saison) -> modèle glb ingéré, "" sinon (repli : teinte de saison du shader, déjà présente).
- Tests : `game/tests/me8_agri_test.gd`, `tools/tests/test_agri_regions.py`.

## Brancher un nouveau modèle / une nouvelle matière
- Texture dédiée (ex. `rice_paddy`) : l'ajouter à `ground_materials.yaml` ; le repli cesse de s'appliquer tout seul.
- glb saisonnier : `cent-ans dn-ingest` l'inscrit au manifeste ; `AgriSeasons.model_for` le rend. Le placement MultiMesh reste à brancher dans `vegetation.gd` (non fait : voir points ouverts).

## Provence verte à l'est du Rhône
Cause : la carte de couleur SS cuite à partir du splat (prairie R 0,86 en Provence contre R 0,45 et fermes G élevées en Languedoc) + écart de saison du shader. Le vert suit exactement la province `prov_provence` (terrain `hills`). Non corrigeable par le parcellaire (invisible au-delà de rig ~ 100). Voir rapport.

## Points ouverts
- Rizières italiennes : attestées seulement à partir du XVe s. ; plaine lombarde = prés irrigués (marcite) ici.
- Rangées d'arbres (oliviers, vergers) : semis natif `core/crates/vegetation`, non touché.
