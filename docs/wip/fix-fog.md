# fix-fog — météo brouillard de campagne

Branche fix/fog (depuis fix/integ). ADR 0596 (amende 0142). TERMINÉ.
- `DepthFog::at_weather`, `Atmosphere.mist` lissé (2,5 s), couleur gris-blanc relayée au shader terrain (`base`/`sky`), données `campaign.weather_fog` de `data/fx/atmosphere.json` + schéma, test `fog_weather_pulls_the_fog_in_and_clear_is_unchanged`.
- Preuve dump (`CENT_ANS_WEATHER=fog`, Paris) : d=150 brume 187→825 (clair) contre 99→282 ; d=40 50→220 contre 26→75 ; couleur 0,63/0,72/0,89 contre 0,76/0,79/0,83.
- Limite : le ciel d'horizon (bleu) n'est pas grisé ; la brume reste globale (foyer de la caméra), pas par pixel.
- Tests client + smoke (10/10) OK.
