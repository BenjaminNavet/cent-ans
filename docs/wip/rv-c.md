# RV-C — ombres de nuages qui dérivent

Branche feat/rv-c (worktree gp-rv-c). Chantier : docs/wip/rv-relief-vivant.md.

## État
- Diagnostic : l'ombre CV1 (`cloud_shadow`, campaign_life) était active (`life_enabled`) mais
  `cloud_shadow_amount` valait 0 par temps clair (règle TB2) et 0,05 sous la pluie : invisible.
- Nouvel include `game/shaders/campaign_cloud_shadow.gdshaderinc` (après campaign_weather) :
  cumulus fbm à domaine déformé (≈ 10 km), couverture régionale lente, plus forte sous la
  météo chargée ; sous les nuées visibles, même champ `wx_cloud_shape` ; décalage selon la
  direction du soleil (nœud Sun) ; fondu au dézoom (taille d'un cumulus à l'écran), en vue
  politique et parchemin ; teinte froide.
- Réglages : clés `cumulus_*` du bloc `clouds` de data/ui/campaign_map.json (schéma à jour),
  posées par CampaignWeatherView.

- Aussi sur la mer (lumière directe et reflet du soleil, dans `light()` de water.gdshader) et
  les imposteurs d'arbres (albédo et transmission) ; cartes d'arbres proches (FC6) non traitées.
- Valeurs : force 0,40, fréquence 0,032 (cellules ≈ 22 km), couverture 0,22-0,5 (0,7 sous la
  pluie), bord 0,10, dérive 0,3 × vent (≈ 0,9 px/s), fondu quand un cumulus < 12 → 4 px d'écran.
- Vérifié : import, smoke OK ; sonde A/B (part d'écran assombrie > 12 %) : 13-26 % en vue
  régionale (rig 150-600) ; 4 planches (hc_shots) : ombres larges lisibles à rig 150-300.

## Points ouverts
- Le sol n'a pas de `light()` : l'ombre multiplie l'albédo (lumière ambiante comprise), avec
  teinte froide. Sur la mer, ombre discrète (la part directe y est faible).
- Les tests tb2/tb6 (« pas d'ombre de nuages par temps clair ») portent sur `shadow_amount_for`
  (nuées de la météo) et passent ; la règle TB2 est de fait levée pour les cumulus (ADR 0167).

## Prochaine étape
Intégration dans feat/rv.
