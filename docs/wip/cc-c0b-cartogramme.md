# CC C0b — cartogramme, généralisation, planche 2D

État : prototype terminé (aucune donnée de jeu modifiée). Commande : `uv run --project tools cent-ans compose preview` (planches `--strength` répétable, plus la géographie réelle).

## Méthode
- `tools/cent_ans_tools/compose/warp.py` : cartogramme de diffusion (Gastner-Seguy-More 2018, flux rapide par DCT) sur la carte actuelle à 1/8 (885 × 763 cellules, emprise lon −11..45, lat 28..66). Densité = (1−strength)·uniforme + strength·densité gaussienne des lieux de `selection.json` (400 lieux, C0a) ; mer = 0,6 × moyenne terrestre ; hors emprise (Oural, steppe) = 0,08. Arrêt quand le CV de la densité diffusée < 3 %. Grilles avant et inverse (inverse par triangulation puis Newton), `forward(lon, lat)` / `inverse(x, y)`, npz dans `data/map/compose/warp_*.npz` (gitignoré). Coordonnées composées = pixels pleine résolution (719 m), origine NO de l'emprise.
- `generalise.py` : un `Frame` (raster composé à l'échelle 0,5) rééchantillonne par déformation inverse : terre (lissage gaussien 14 km, simplification 5 km, îles < 5 000 km² supprimées sauf 26 îles gardées, gonflées à 3 500 km² ou créées), crêtes (`massif_mask` de navgrid sur le MNT demi-résolution + collines ≥ 650 m, squelette, élagage, simplification), provinces (labels échantillonnés au plus proche, fusion `merged_from`, provinces retirées = hors jeu). Fleuves : ≈ 40 noms (clés + plus longs dans l'emprise), déformés par `forward`, lissés (Chaikin).
- `preview.py` + `commands/compose.py` : planches 1600 px.

## Mesures (400 lieux, sigma 150 km)
| | CV plus proche voisin avant | après |
|---|---|---|
| strength 0,5 | 0,67 | 0,50 |
| strength 1,0 | 0,67 | 0,45 |

Facteur linéaire moyen (0,5 / 1,0) : France 1,68/2,10 ; Angleterre 1,39/1,64 ; Italie 1,24/1,44 ; Ibérie 1,23/1,34 ; Empire 1,63/2,00 ; Scandinavie 1,06/0,94 ; Russie occ. 1,12/1,03 ; Balkans 1,26/1,42 ; Anatolie 1,16/1,20 ; Maghreb 0,85/0,57.
Calcul : cartogramme ≈ 1,5 min (≈ 450-600 pas), planche ≈ 10 s.

## Planches (hors dépôt, ~/dev/cent-ans-raw/compose/)
`planche_s0.50.png`, `planche_s1.00.png`, `planche_reelle.png`, `rapport.json`.

## Limites
- Gain d'uniformité modeste (le mélange avec l'uniforme et le lissage 150 km limitent) ; sigma plus petit = plus régulier, plus déformé.
- Pas d'épingles d'auteur ; mer compressée (Méditerranée très aplatie).
- Collines (traits clairs fins) se confondent avec les frontières sur la planche ; fleuves en tronçons (≈ 150).
- Frontières rastérisées (pas de vecteur) ; rayons de `massif_mask` divisés par 2 (demi-résolution).
- Les 26 îles gardées : plusieurs sont « créées » (disque) car absentes du masque déformé.
