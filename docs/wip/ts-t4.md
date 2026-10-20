# TS-T4 villes (halo chaud + fumées) : FAIT

Branche `ts/t4`. Module `client/src/campaign/towns/glow.rs`, réglages `data/fx/town_glow.json`
(schéma `fx_town_glow`, test `tools/tests/test_town_glow_schema.py`). Dump : clé `glow` de `campaign_towns`.
Vérifié : clippy, 1336 tests, smoke 10/10, dump offscreen (400 halos, 12 panaches à d=200 sur Paris).
Restes : jugement à l'œil (rayon du halo, taille des panaches), pas de nuit (la campagne n'a pas d'heure : saison seule).
