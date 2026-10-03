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

## Prochaine étape
Import, smoke, captures régionales (≤ 4), réglage des valeurs ; impostors d'arbres et eau.
