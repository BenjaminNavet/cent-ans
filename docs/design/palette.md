# Bible de palette (ADR 0755)

Une seule palette pour la campagne et la bataille. Données : `data/fx/palette.json` (schéma `fx_palette.schema.json`). Lecteur : `client/src/palette.rs`. Mesures du juge DA (`out/juge/iter-02-art.md`, 5,3 / 10) : deux mondes colorimétriques (campagne gris-vert froid, bataille jaune-vert saturé et ciel bleu vif), aube indiscernable du midi, crépuscule sale.

## Référence
- **Saison de référence : printemps. Heure de référence : midi.** Toute autre saison ou heure est un écart dérivé.
- Herbe de référence (albédo sRGB) `[0.34, 0.50, 0.22]` : teinte 94°, saturation 0,56. L'été glisse vers le jaune (teinte -16°), l'automne vers l'ocre (-49°), l'hiver est quasi neutre et clair.
- Cible mesurée sur capture de bataille à midi, herbe d'été : teinte 82-106°, saturation 0,22-0,42, valeur 0,36-0,56. Avant : teinte 73°, valeur 0,48 ; après (clair, été) : 67°, 0,57 (le jaune des couches de texture reste à réduire, voir ADR).

## Matières (printemps, midi, albédo sRGB)
Sol : herbe `0.34 0.50 0.22`, herbe sèche `0.55 0.48 0.25`, terre `0.42 0.33 0.23`, boue `0.30 0.24 0.18`. Pierre : craie `0.74 0.71 0.64`, pierre `0.58 0.55 0.50`, pierre sombre `0.38 0.36 0.34`, ardoise `0.30 0.32 0.36`. Toits : tuile `0.55 0.30 0.20`, chaume `0.60 0.50 0.30`. Bois et feuillage : écorce `0.26 0.20 0.15`, feuillage `0.22 0.36 0.14`. Cuir : `0.32 0.20 0.12`, sombre `0.18 0.12 0.08`. Textile : laine écrue `0.74 0.68 0.54`, lin `0.82 0.78 0.68`. Eau `0.16 0.28 0.32`. Aucun matériau ne devrait sortir de ces familles sans être rattaché ici.

## Ciel
Zénith `0.30 0.50 0.80` (jamais le bleu vif 0,2 / 0,42 / 0,78 saturé), horizon `0.72 0.80 0.88`, brume `0.70 0.76 0.84`. En bataille claire la saturation du panorama est ramenée à 0,9.

## Étalonnage commun
Contraste 1,05, saturation 0,97, saturation des ombres 0,65, tolérance 0,05 / 0,1. La campagne (`atmosphere.json` `campaign.seasons.*.grade`) et la bataille (saison x météo claire) y sont tenues par le test `palette::campaign_and_battle_share_the_common_grade`. Les noirs de la campagne ne sont plus relevés vers un gris-bleu (lift 0,004 / 0,005 / 0,008 au lieu de 0,01 / 0,012 / 0,02).

## Heures de bataille (`atmosphere.json` `time_of_day`)
Six préréglages, un par phase du cœur. Le soleil se règle en élévation, azimut, énergie et **température de couleur** (`sun_kelvin`), l'air en teinte de brume, diffusion, lueur côté soleil, visibilité et ambiance (`air`).
| Heure | Élévation | Soleil | Air |
|---|---|---|---|
| Aube | 8° | 3100 K, énergie 2,5 | brume basse rose doré, visibilité x0,5, ciel « dawn » avec disque |
| Matin | 24° | 4500 K | brume bleu pâle, x0,85 |
| Midi | 32° | 5600 K | neutre |
| Après-midi | 22° | 4700 K | lueur dorée côté soleil |
| Crépuscule | 9° | 3300 K, énergie 2,6 | brume orangée, ambiance violette x1,5, ciel « dawn » |
| Nuit | 20° | 9500 K, énergie 0,2 | brume bleu nuit, ciel x0,12 |
Contrôle automatique : l'éclairement au sol (énergie x sin élévation) de l'aube et du soir reste >= 25 % de celui de midi (test `every_hour_has_its_own_light_and_the_dusk_is_not_dark`).

## Où se trouve quoi
Sol et herbe de bataille : `ground_of` (`palette.json` `ground`). Étalonnage : `atmosphere.json` (saisons, météo, heure) puis la passe `grade.wesl` en bataille, `post.wesl` en campagne (réglages `campaign_post.json`). Brouillard (ADR 0596) : inchangé.
