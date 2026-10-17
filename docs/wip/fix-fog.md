# fix-fog — météo brouillard de campagne

Branche fix/fog (depuis fix/integ). ADR 0596.
- Fait : `DepthFog::at_weather`, `Atmosphere.mist` lissé, données `campaign.weather_fog`, schéma, test unitaire.
- Reste : captures avant/après (clair puis `CENT_ANS_WEATHER=fog`, d≈150 et d≈40), tests complets, smoke.
