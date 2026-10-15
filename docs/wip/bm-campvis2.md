# bm-campvis2 — parité visuelle campagne (suite de campvis)

## Fait
- Ombres portées : relief projeté (`terrain_shadow.wesl`), sol récepteur (`sun_shadow`), cascades à la distance caméra (`terrain/shadows.rs`, port de `campaign_atmosphere.gd apply_distance`, données `campaign_lighting.json § shadows`), coupées au-delà de 650. Le lot settings garde la main finale (`SunBase` × `map_shadow_range`, case Ombres).
- GA4 macro-variation (`campaign_terrain_textures.json § macro`) et TB6 massifs sombres (`campaign_map.json § forest_masses`) dans `terrain.wesl`.
- Parchemin : `pm_output` porté (papier surtout en émission) : vue 1200 grise comme Godot, plus pastel.
- Politique : `provinces/shaders/fill.wesl` + `drape.rs` (hors dossier, minimal) : teinte multiplicative à luminance constante (`colorized` de terrain.gdshader) ; c'était la cause principale du sol délavé (moyenne 136 → 103 à 400).
- Caméra : formule de tangage identique à Godot (35,9° à 400, 7080 × 0,45 → 2600) ; l'« horizon » de Godot à 400-120 est la perspective aérienne (lot weather).
- Trous de tuiles près de Paris (25-60) : non reproduits (`missing_pages` 0, 2 vues) après le correctif `NoFrustumCulling` de campvis.

## Mesures (zone 480×370 au centre, Beauce, été, ciel clair ; Godot / avant / après)
- 400 : moyenne 103,93,61 / 136,120,72 / 103,102,64 ; écart-type lum. 16,5 / 13,3 / 16,0 ; HF 4,7 / 2,5 / 4,2.
- 1200 : 126,122,108 / 153,137,109 / 124,120,100 ; 15,0 / 12,5 / 13,4.
- 120 : 100,87,56 / 116,104,67 / 96,93,63 ; 17,3 / 7,7 / 9,7.

## Restes
- Sol de près (120-30) : patchwork Godot plus contrasté (champs bruns, bosquets) ; à porter : `terroir_landuse` + `landuse` (texture exposée par vegvis `countryside::landuse::LanduseTexture`, R vigne G sécheresse B bocage), parcellaire V2b (`field_at`, haies, chemins), `hb_clearing_splat`, `fp_parcels` (fine_parcels.gdshaderinc, textures `tx_campaign_parcels_*`).
- Arbres (vegvis) : `shade_foliage` de `billboard.wesl` devrait aussi multiplier le soleil par l'ombre portée.
- Frontières de royaume plus fines que Godot (largeurs du ruban de `provinces/borders.rs`, lot provinces) ; pas de port dans le shader de terrain (le ruban existe déjà avec hachures).
- Teinte politique : Godot fond vers un aplat au dézoom (`political_flat_gain`) ; non porté (le multiplicatif ne connaît pas la luminance du sol).
