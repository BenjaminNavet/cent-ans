# 0192 — Météo de campagne cuite une fois par tour
Date : 2026-10-08. Statut : acceptée. Chantier FL (fluidité de la carte).

## Contexte
`wx_sample` (campaign_weather.gdshaderinc) calcule à chaque pixel du sol et du plan de nuées la
frange irrégulière entre provinces : 2 fbm (6 bruits) pour déformer, puis 3 lectures de province
et 3 lectures du masque météo. Le résultat ne dépend que du point de carte et du masque du tour.
La météo coûte 2 à 6,5 ms par image ; 84 % de la carte est sous une météo en automne 1337, donc
un saut « hors météo » n'aide pas (essayé, annulé).

## Décision
CampaignWeatherView cuit `wx_sample` pour toute la carte dans une SubViewport (1 texel pour
4 px carte, 1792 × 1536 RGBA8, `weather_field_bake.gdshader`), rendue une fois (`UPDATE_ONCE`)
à chaque nouveau masque. Le sol et les nuées lisent cette texture filtrée (`weather_field_on`).
Le banc `--bench-set=prop:weather_view.use_field=false` rétablit le calcul par pixel.

## Conséquences
- Plein écran HiDPI (0,4) : 26,3 → 25,5 ms en A/B en processus (−0,8 ms) ; rendu identique au
  bruit de capture près (`tests/fl_weather_field_shot.gd`).
- 11 Mo de mémoire vidéo. La frange se recalcule à chaque tour, pas en continu (elle était déjà
  fixe dans un tour). Toute retouche de `wx_sample` passe aussi par le shader de cuisson (même include).
