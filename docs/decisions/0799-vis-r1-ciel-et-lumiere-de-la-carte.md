# 0799 — Ciel, lumière et air de la carte de campagne (VIS-R1)

Date : 2026-10-29. Statut : acceptée (numéro à attribuer à la fusion). Amende les ADR 0156, 0710 et 0755 ; touche 0596 et 0124 sans les contredire.
Lot VIS-R1 de la boucle juge 2 (audit `docs/audits/2026-10-29-visuel-campagne.md`, refonte R1). Détails : `docs/wip/vis-r1.md`.

## Contexte
L'audit constatait un mur gris à la place du ciel (fond `Color::srgb(0.74, 0.76, 0.76)`, caméra à 30° au plus près : l'horizon n'entrait jamais dans le cadre) et une lumière froide
(soleil 1,5 × sin 20° ≈ 0,51 contre un ciel bleu à 0,56 : le ciel dominait), d'où un sol gris-bleu (saturation 0,17, valeur 0,36).

## Décisions
1. **Ciel : dôme dégradé, pas l'`Atmosphere` de Bevy 0.20.**
   - `Atmosphere` / `AtmosphereSettings` exigent `Hdr` sur la caméra (`#[require(Hdr)]`). La passe post de la carte (`terrain/post.wesl`) tourne après le tonemap sur une cible LDR, avec sa lecture de profondeur et son étalonnage : passer la caméra en HDR les remet en cause tous.
   - Il n'y a pas de champ d'échelle scène → mètres en 0.20 : 1 u = 719 m et le relief ×4,3 imposeraient de mettre à l'échelle les rayons et les densités du milieu à la main.
   - Le dôme (`weather/dome.rs`, `dome.wesl`) coûte un tirage d'une sphère à la profondeur 0 (aucune écriture de profondeur) ; il ne remplit que ce que rien n'a dessiné. L'horizon prend la **teinte vivante de la brume de caméra** (`Atmosphere::color` : saison, perspective aérienne, brouillard), le zénith s'en déduit par le rapport zénith / horizon de `palette.json` `sky`, ramené vers l'horizon par météo couvrante. Réglages : `data/fx/campaign_sky.json` (schéma `fx_campaign_sky.schema.json`). Dump : `campaign_weather.atmosphere.sky` (`kind`, `visible`, `overcast`, `horizon`, `zenith`).
   - Le prototype `Atmosphere` n'a pas été mesuré : le critère de coût n'a pas été nécessaire pour décider.
2. **Caméra** : inclinaison en données (`data/ui/camera_feel.json` : `pitch_near_deg` 20 à d 22, `pitch_mid_deg` 28 à d 150, linéaire jusqu'à 70° à 2 600). Avec un champ de 55°, le rayon du haut d'écran passe au-dessus de l'horizon jusqu'à d 60 (test du rig).
3. **Lumière** : soleil de fin de matinée (32-36°, énergie 1,8-2,4, teinte ≈ 5 200 K), ambiance moins bleue et plus faible (0,75 0,80 0,90, énergie 0,40), bornes d'élévation 26-40°. Test : l'éclairement solaire au sol (énergie × sin élévation) vaut au moins `targets.campaign_sun_sky_ratio_min` (1,8) fois l'énergie d'ambiance, pour chaque saison et chaque variation de tour.
   Le crépuscule de fin de tour reste ; **pas d'aube rose** de début de tour (décision de mise en scène).
4. **Brume unique** : la passe post porte seule la brume de vallée et la perspective aérienne sur la profondeur, mer comprise (`valley_mist.sea_share` 1,0). La nappe peinte de vallée du shader de sol est neutralisée (`campaign_map.json` `morning_mist.opacity` 0). Plafond du voile lointain par temps clair : `campaign_post.json` `aerial.clear_max` 0,3. Teintes de brume alignées sur l'horizon du ciel.
5. **Palette** : `data/fx/palette.json` reçoit un bloc `campaign` (albédo des matières de la bible, ordre de valeur) et les cibles de mesure ; tests dans `client/src/palette.rs`.
6. **Météo** : le premier tour d'une nouvelle partie est dégagé (`first_turn_clear` de `data/rules/campaign_weather.json`, règle dans `core/` : `sim_campaign::weather::compute_weather`).

## Conséquences
- Un ciel et un horizon visibles en vue basse ; les couleurs d'air sont désormais une seule chaîne (brume de caméra → ciel → perspective aérienne).
- La bible (c) n'est pas entièrement cohérente avec son propre ordre de valeur : le labour (0,45 0,36 0,26) est plus sombre que le pré en luminance, la tuile aussi. Le test ne vérifie que mer < forêt < pré < blé < chaume < neige, et forêt < toute culture.
- Les nuages de saison du dôme ne sont pas faits : cirrus et cumulus existants restent.
- Le sol reste le « camouflage » de facettes : c'est le lot R3.
