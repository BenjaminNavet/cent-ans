# Texture / material artist — état des lieux (09/10, lecture seule)

## 1. État actuel
- `game/assets/textures/` : 174 Mo (terrain 52, battle 48, vegetation 34, buildings 29, water 3,8, fx 3,1, landmarks 2,5).
- **Campagne** : GA4 `terrain_albedo_array.jpg` (7 × 2048²) + normales (`campaign_textures.gd`, `campaign_terrain_textures.json`) ; HB2 `hb_ground_albedo_array.jpg` (27 × 1024²) + `hb_ground_normal_array.jpg` (27 × 512², B = rugosité), fal flux-2-pro 1,10 $ ; colormap SS (ADR 0142) `colormap_bc1_{0..3}.bin` 14336×12288 BC1 (~39 Mo, ~117 Mo VRAM, ~360 m/texel).
- **Bataille** : `ground_albedo_array.jpg` 13 × 2048² Poly Haven, répétition 3,5-9 m ; normales 1024² ; splatmap 4 m/texel ; détail proche PO4 1,4 m effacé à 32 m ; herbe et feuilles en BC7.
- **Bâtiments/lieux** : tableaux de 16 couches en **512²** (sources 2k dans le même dossier).
- **Modèles générés** : 2 463 glb (1,3 Go), 2 199 textures extraites (1 517 en 1024², 632 en 512², 3 en 2048²).
- **Imports** `assets/textures` : 119 `.import`, 75 en VRAM, 44 sans perte, 10 en BC7 ; tous les tableaux en BC1, **normales comprises**.

## 2. Forces
- Chaînes reproductibles (`build_textures.py`, `assets ground-materials`, `geo colormap`, `water_procedural.py`) ; réglages en données.
- `Texture2DArray` ; anti-répétition (double échelle tournée, `ga_macro`, détail proche).
- Raccord HB2 soigné (coupe d'erreur minimale, luminance égalisée à 0,18) ; licences tracées.

## 3. Faiblesses
1. **Textures des modèles générés non compressées** : 2 177/2 199 en `compress/mode=0` + `detect_3d/compress_to=1` jamais déclenché → ~5,3 Mo VRAM par 1024² au lieu de 0,7 Mo ; export gonflé.
2. **Normales en BC1** dans 5 tableaux (blocs en lumière rasante ; rugosité HB écrasée) ; sources JPEG.
3. Eau sans mipmaps (`water/*`) → scintillement au loin.
4. Texel density incohérente : bâtiments 512² contre 1024-2048² ailleurs (ADR 0211 D3 prévoit 1024-2048²).
5. Colormap : taches forêt/champ à une seule échelle (« léopard »), frontière de steppe droite vers 26-28° E, Islande/Norvège trop vertes, sable uniforme en Afrique du Nord.
6. Fichiers inutiles importés : replis 1k sans perte, `vegetation/dn_cards/`, sources 2k de `buildings/` (~25 Mo), `__pycache__`.
7. `battle_ground_layers.json` : « meadow » = `rocky_terrain_02`, « dirt » = `coast_sand_01`.
8. Albédos TRELLIS sombres avec lumière cuite, compensés par `albedo_gain` 2,4 (ADR 0214) ; `target_albedo_mean` non lu.
9. Détail proche de campagne flou (SS4 reporté).
10. Splatmap de bataille 4 m sans mipmaps : transitions molles.

## 4. Améliorations
| # | Action | Impact | Effort | Coût | Dépend de |
|---|---|---|---|---|---|
| 1 | Forcer `compress/mode=2` (+ `normal_map=1`) sur les imports des modèles | Fort | S | 0 | perf, build |
| 2 | Tableaux de normales en BC5/BC7, rugosité HB séparée, sources PNG | Moyen-fort | S-M | 0 | graphics |
| 3 | Mipmaps + VRAM sur `water/*` | Moyen | S | 0 | FX |
| 4 | Colormap multi-échelle, steppe bruitée, nord désaturé, sable varié (`colormap_style.yaml`) | Fort | M | 0 | géo, DA |
| 5 | Bâtiments/lieux en 1024² | Moyen | S | 0 (+~45 Mo VRAM) | perf |
| 6 | « Délight » des albédos TRELLIS, suppression de `albedo_gain` | Moyen | M | 0 | tech art |
| 7 | Nettoyage (sources 2k, `dn_cards`, `.gdignore`, `__pycache__`) | Faible | S | 0 | build |
| 8 | Vraies prairies et terre battue en bataille | Moyen | S | 0 | level |
| 9 | Détail proche de campagne SS4 (4-6 matières) | Moyen | M | ~1-2 $ | joueur |
| 10 | Splatmap 2 m + mipmaps | Moyen | M | 0 | bataille |
| 11 | Hex-tiling au-delà de 30 m | Moyen | M | 0 | graphics |
| 12 | Atlas de fleurs | Faible-moyen | S-M | 0 ou fal | végétation |

Avant le point 1 : mesurer la VRAM réelle d'une bataille et d'une vue de ville.
