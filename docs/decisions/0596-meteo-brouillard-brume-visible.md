# 0596 — La météo « brouillard » de campagne rend la brume visible

Statut : accepté (17/10, lot fog des correctifs post-playtest). Amende l'ADR 0142.

## Contexte
L'ADR 0142 a établi que la météo ne touche pas la couche de brume de distance (`weather/atmosphere.rs`) : la vraie cause de l'aspect délavé était le plafond `weather_mist_max` = 0,07. Conséquence : `CENT_ANS_WEATHER=fog` donnait la même brume qu'en temps clair (75→330 contre 112→495, aucun brouillard perceptible, playtest carte-campagne du 17/10).

## Décision
- Seule exception à 0142 : quand la province sous le foyer de la caméra est en météo « brouillard », la couche unique `DistanceFog` est resserrée : début et fin multipliés (`begin_scale` 0,5, `end_scale` 0,3), densité relevée (0,95), courbe adoucie (1,0), couleur gris-blanc (0,78 0,8 0,82) légèrement teintée du ciel (`aerial_keep`), diffusion solaire éteinte.
- Force = `min_strength` + (1 - `min_strength`) x intensité de la province ; entrée et sortie sont lissées (`ease_seconds` 2,5 s).
- Paramètres dans `data/fx/atmosphere.json` (`campaign.weather_fog`), schéma `fx_atmosphere.schema.json`. Force 0 = résultat strictement identique à avant (`DepthFog::at`) : temps clair, pluie, neige et orage ne changent pas, le plafond de 0142 reste en vigueur.

## Conséquences
- Le relief lointain s'estompe réellement en brouillard ; la brume reste lissée au passage d'une province à l'autre (le foyer de caméra sert de référence, pas de brouillard par pixel).
- Test unitaire : brume brouillard > clair, clair inchangé.
