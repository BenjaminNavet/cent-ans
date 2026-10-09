# ME1 — mer vivante (branche dn/me1-mer)

État : shader et données écrits, test headless en cours de validation.
- `data/fx/sea_life.json` (+ schéma `fx_sea_life.schema.json`) : saturation max 0,40, liseré de ressac à largeur écran minimale, fonds clairs/caustiques, crêtes du large, estrans (8 zones, marée 150 s), sillage.
- `game/shaders/sea_life.gdshaderinc` inclus par `water.gdshader` ; anti-damier : couches GA4 et RC5 tournées, gauchies, modulées par du bruit basse fréquence, estompées plus tôt au dézoom.
- `SeaLife` (`scripts/map/sea_life.gd`) pose les uniformes (appelé par `Sea.setup`) et construit les sillages (`fleet_wake.gdshader`, un ruban par navire dans `ArmyFigures._build_fleet`).
- `sea_basins.json` : teinte Atlantique adoucie (0,95/1,10/1,22).
- `--no-sea-life` : A/B.
Reste : captures de contrôle par l'orchestrateur ; poissons sautants / dauphins glb (non faits).

Fin (10-09) : motif régulier supprimé. `sea_wave_fade` (période à l'écran : 0 sous ~4 px, plein au-delà de ~14 px) sur le contraste de houle et de vaguelettes ; longue houle fondue sur la période écran, directions dérivant par bruit ; phases de `swell()` déformées par bruit (plus de sinusoïde pure). Capture de contrôle : `tests/me1_sea_shot.gd` -> ~/dev/cent-ans-raw/dn/me1/final.png. me1_sea_life_test et smoke OK.
