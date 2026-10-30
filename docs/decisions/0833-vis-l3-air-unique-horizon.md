# 0833 — VIS-L3 : une seule brume (passe post) et nuages sur le dôme

## Contexte
Le juge visuel (iter. 3, G1-3) relevait un horizon « fantôme » : la brume de profondeur de la caméra (`DistanceFog`, courbe Godot) voilait le sol, la perspective aérienne de la passe post voilait une seconde fois (sol et mer, avec des départs différents), la mer n'avait pas la même transmittance que la terre, et le ciel était une bande de lait sans nuages.

## Décision
- Une seule passe d'air : la courbe de brume de caméra (`DepthFog` : begin, end, courbe, densité, calculée par `weather/atmosphere.rs`) est appliquée par `terrain/post.wesl` d'après le tampon de profondeur, à terre comme en mer (même formule donc même transmittance à profondeur égale). Le `DistanceFog` de la caméra reste posé avec alpha 0 (aucun second voile ; les ombres de scatter solaire sont portées par ailleurs).
- Couleur de la brume : couleur de brume vivante tirée vers le bleu de ciel de la perspective aérienne (`campaign_sky.json` `haze_sky_share`), proche -> lointain. L'horizon du dôme prend la même couleur (`SkyLook::haze_color`), donc ciel, terre et mer se rejoignent sans raccord ; juste au-dessus de la ligne d'horizon le bleu de ciel revient (`band_saturation`, `gradient_exponent`).
- Nuages de campagne : couche procédurale (fbm, bords effrangés, côté éclairé vers le soleil) sur le dôme, réglée dans `campaign_sky.json` `clouds`, couverture accrue par la couverture météo. Coût : un fragment de ciel seulement.
- La part de ciel de la passe post (`aerial.sky_share`) passe à 0 : le dôme porte seul la teinte du ciel.

## Conséquences
- Le masquage des plaques, étiquettes et bannières d'armée au-delà de l'horizon (spec L3) relève de `markers/` et `armies/` (hors zone de ce lot) : ils doivent lire `Atmosphere.fog` (`DepthFog::amount(distance)`, seuil 0,6) ; non fait ici.
- Les matériaux qui ne lisent pas la profondeur (UI) ne sont pas voilés, comme avant.
