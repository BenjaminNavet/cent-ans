# bm-campvis2 — parité visuelle campagne (suite de campvis)

État :
- Ombres portées du soleil : le relief projette (prépasse `terrain_shadow.wesl`, même hauteur affichée), le sol les reçoit (`sun_shadow` de `terrain.wesl`), cascades dimensionnées à la distance caméra (`terrain/shadows.rs`, port de `campaign_atmosphere.gd apply_distance`, données `data/fx/campaign_lighting.json § shadows`) ; coupées au-delà de 650. Le lot settings garde la main finale (`SunBase` × `map_shadow_range`, case Ombres).
- GA4 macro-variation (teinte + luminance, `campaign_terrain_textures.json § macro`) et TB6 massifs sombres de la carte de couleur (`campaign_map.json § forest_masses`) portés dans `terrain.wesl`.
- Parchemin : `pm_output` porté (papier surtout en émission, ¼ dans l'albédo éclairé) au lieu d'un mélange 0,95 : vue 1200 moins pastel.
- Caméra : formule de tangage identique à Godot (35,9° à 400, mêmes bornes 7080 × 0,45 → 2600) ; l'« horizon » de Godot à 400-120 est la perspective aérienne (lot weather), pas un écart de cadrage.

Prochaine étape : mesures 400/1200, relief ombré (Auvergne), commit final.
