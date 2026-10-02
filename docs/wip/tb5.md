# TB5 — mer et côtes (note de reprise)

Spec : `docs/design/2026-10-02-campagne-tob.md` § 3 « TB5 ». Branche `feat/tb5`, worktree
`/Users/jean_hubert/dev/gp-tb5`. Rendu Godot seulement (`core/` intact). Pas de fal.ai (ADR 0152).
Arbitrages : ADR 0155 (`docs/decisions/0155-cotes-et-mers-par-region.md`).

## État
- [x] Squelette : cette note, `game/tests/tb5_coast_test.gd`.
- [x] 1. Falaises (craie, granite, roche) et plages (sable, galets) : `data/map/coast_types.json`
  (régions en px carte, règle de pente, couleurs), schéma `coast_types.schema.json`,
  `tools/tests/test_coast_types.py` ; `CoastLook` (`game/scripts/map/coast_look.gd`) cuit la texture
  de géologie et pose les réglages ; `coast_band.gdshaderinc` + `coast_common.gdshaderinc`, un
  crochet `coast_band(...)` dans `terrain.gdshader`, une ligne dans `terrain_builder.gd`. Matières
  procédurales : aucune texture ajoutée, pas de ligne `CREDITS.md`.
- [x] 2. Mers par bassin : `data/map/sea_basins.json` (Atlantique, mer du Nord et Manche,
  Méditerranée, Baltique ; hors bassin : mer par défaut), schéma `sea_basins.schema.json`,
  `tools/tests/test_sea_basins.py` ; `SeaBasins` (`game/scripts/map/sea_basins.gd`) ;
  `sea_basins.gdshaderinc` inclus par `water.gdshader` (teinte, clarté, clapot, longue houle,
  écume, moutons). La saison TB1 s'applique par-dessus (`grey_scale` par bassin).
- [x] 3. Ressac : `coast_swash` (`coast_common.gdshaderinc`), appelée par `water.gdshader` (côté
  eau) et par la bande côtière (côté plage) ; bloc `swash` de `coast_types.json`.
- [x] `game/tests/tb5_shot.gd` : captures (fichiers écrits, non lus), `--stats`, `--bench`.
- [x] `tb5_coast_test`, `tb1_seasons_test`, `smoke.gd`, pytest (`test_coast_types`,
  `test_sea_basins`) passent.

## Prochaine étape
Lot livré. Reste à la session principale : capture de contrôle, puis réglage à l'œil dans
`data/map/coast_types.json` (`band`, `types`, `swash`) et `data/map/sea_basins.json` (`look`).

Capture : `godot --path game --resolution 1600x900 --script res://tests/tb5_shot.gd --
--out=<dossier> --season=summer [--only=douvres,etretat,raz,landes,manche,mer-du-nord,atlantique,mediterranee,mediterranee-large] [--distances=90,40]`.
Positions (px carte) : Douvres / South Foreland 2153,2840 ; Étretat 2012.6,3047.5 ; pointe du Raz
1476.5,3218 ; Landes (Mimizan) 1740,3867.2 ; Manche 2030,2950 ; mer du Nord 2400,2500 ;
Atlantique 1000,2800 ; Méditerranée 2500,4200 (golfe du Lion) et 2400,4500 (large).
Étretat fait face au nord-ouest : la paroi est cachée par le plateau vu du sud ; Douvres et les
Landes se lisent mieux.

## Chiffres (02/10, été, nuées masquées)

`ss_shot.gd --stats` (moyenne RVB du tiers bas), avant → après :

| Lieu (`--at`) | Distance | Avant | Après |
|---|---|---|---|
| Douvres 2148.5,2843.2 | 400 | 76 72 47 | 71 65 41 |
| Douvres | 90 | 26 43 42 | 15 24 24 |
| Douvres | 40 | 24 42 41 | 14 26 26 |
| Étretat 2012.6,3046.6 | 40 | 90 79 46 | 107 92 57 |
| Landes 1738.8,3867.2 | 40 | 90 76 51 | 108 85 54 |
| Atlantique 1000,2800 | 400 / 90 | 8 15 20 / 11 22 25 | 11 19 24 / 14 26 31 |
| Manche 2030,2950 | 90 | 22 41 41 | 11 20 21 |
| Mer du Nord 2400,2500 | 90 | 23 42 41 | 14 26 26 |
| Méditerranée 2500,4200 | 400 / 90 | 7 11 16 / 7 11 16 | 21 45 57 / 21 44 57 |

À Douvres, le tiers bas est surtout de la mer (Manche assombrie). Avant le lot, la Manche et la
mer du Nord, peu profondes, sortaient plus claires que l'Atlantique et la Méditerranée.

`tb5_shot.gd --stats` (pixels très changés par le lot, donc cœur de la bande côtière) : Étretat à
15 : 201 173 136 (craie sous lumière dorée) ; Douvres à 40 : 160 112 79 (craie sous le voile du
brouillard de guerre, partie jouée en France) ; Landes à 15 : 169 114 75 (sable). La bande couvre
0,2 à 0,6 % de l'image à 40. Hiver à 90 : Manche 40 49 58 (gris acier TB1 conservé),
Méditerranée 32 51 67.

Mouvement en 3 s (`TB5 motion`, part de l'image qui change, sans → avec le lot) : Landes à 40 :
3,5 → 12,4 % ; Étretat à 40 : 3,0 → 5,4 %.

Bench : machine chargée (autres agents), mesures non concluantes. `ss_shot.gd --bench` à Douvres,
deux passes : avant 26,7 / 18,8 ms (400) et 18,8 / 22,1 ms (90) ; après 27,7 / 29,3 (400) et
29,7 / 22,7 (90). `tb5_shot.gd --bench` (sans / avec dans le même processus, médiane de 5 passes) :
Douvres 400 : 22,3 / 17,2 ; Douvres 90 : 34,4 / 37,0 ; Landes 400 : 29,2 / 30,3 ; Landes 90 :
21,4 / 24,5 ; Atlantique 400 : 20,6 / 20,2 ; Atlantique 90 : 23,9 / 23,2. L'écart (−5 à +3 ms) est
dans le bruit ; à refaire sur machine calme.

## Points ouverts
- Bench à refaire sur machine calme (`tb5_shot.gd --bench` avec `--disable-vsync`).
- Rendu non jugé à l'œil (aucune capture lue) : largeur de bande (`band.cliff_px` 1,0,
  `beach_px` 0,6), blancheur de la craie, teinte de la Méditerranée (peut-être trop verte sur le
  plateau du golfe du Lion), moutons de l'Atlantique (`whitecaps` 0,22), ressac (période 7 s).
- La ville de Douvres est dans la vallée de la Dour (sonde à 23 m) : plage de galets ; les
  falaises sont de part et d'autre. Le test prend South Foreland (2153,2841).
- Seuils de falaise (36 / 52 m) réglés sur six lieux ; d'autres cordons de dunes ou côtes basses
  à plateau proche peuvent mal tomber.
- Toutes les côtes de la carte reçoivent la bande (roche et sable par défaut hors région), fjords
  de Norvège et Méditerranée compris.
- La mer peinte sur le terrain et le fond marin ne prennent pas la teinte du bassin.
- La vue parchemin recouvre la bande et les bassins (non vérifié en rendu).
- `smoke.gd` affiche déjà, hors lot, des erreurs `Rect2 size is negative`
  (`settlement_layer.gd:1033`) et un `Parse JSON failed` (`hb_ground.gd:33`).
- Nouvelles classes `CoastLook`, `SeaBasins` : relancer `godot --headless --path game --import`
  après fusion. `terrain.gdshader` : 25 échantillonneurs (24 avant).
