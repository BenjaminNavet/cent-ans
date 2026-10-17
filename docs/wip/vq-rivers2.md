# vq-rivers2 : rivières, liseré clair, berge mouillée, brume (carte de campagne)

État : fait (17/10), à fusionner.

## Fait
- Liseré clair supprimé : l'écume de berge est décalée vers l'intérieur (plus sur le bord), part réglée par `look.bank_foam` ; le bord de l'eau fonce vers `look.wet_color` (`look.bank_dark`, à partir de `bank_wet_start`).
- Bande de sol mouillé hors du ruban : le ruban est élargi de `look.wet_band_px` px écran (sommets, `river.wesl`), la bande est `wet_color` avec alpha dégradé jusqu'à 0 (`wet_band_alpha`, atténuée au loin). Aucune modification de `terrain.wesl`.
- Brume de distance : l'eau applique le `DistanceFog` de la caméra comme le sol (`camera_distance_fog`, diffusion solaire `wx_scatter` comprise ; pas de variation par pixel du ton d'horizon), part `look.fog_strength`.
- `river_fine.wesl` : reflet de ciel de base 0,12 -> 0,04 (cohérent avec le palier moyen).
- `CENT_ANS_RIVER_LOOK_OFF=1` neutralise aussi les nouveautés.
- Schéma `river_display.schema.json` à jour.

## Reste
- Brume : pas de teinte aérienne par pixel (le sol l'a via `aerial_base`), seulement la couleur du `DistanceFog` centrale.
- Lacs : pas de bande mouillée (feuille sans bord de ruban), brume appliquée.
