# SL — Sol « satellite » de la carte de campagne

Date : 2026-09-30. Statut : conception validée par le joueur.

## Problème

La carte de campagne 3D se lit comme une carte IGN : le relief ombré domine, les textures de sol (splat 4 canaux + tableaux 2k, GA4) sont quasi invisibles (écart mesuré 0,6–4/255). Champs, routes et lacs ne ressortent à aucune hauteur de caméra. Le joueur constate le défaut **à toutes les hauteurs** de la vue 3D (< 1200).

## Cible

Rendu type Total War (Attila / Pharaoh) : patchwork agricole brun / vert / doré lisible dès la vue large, routes de terre claires, lacs avec rives et roselières ; le relief se lit par la lumière, pas par un aplat grisé. Cohérent avec la direction semi-réaliste (ADR 0136).

## Approche retenue

Une **pyramide de couleur (albédo) précalculée hors ligne** remplace la couleur procédurale du sol en vue large et moyenne ; les textures 2k ne donnent plus que le grain de près. Écartés : retouche du seul shader (coût GPU déjà maximal, ~40 ms, motifs répétitifs de loin) ; repeinte IA de la carte (raccords, routes et parcelles inventées, coût incertain).

## 1. Données — `cent-ans geo colormap`

Module `tools/cent_ans_tools/geo/colormap.py`, sous-commande `cent-ans geo colormap`.

- **Découpage** : identique à la pyramide de relief (ADR 0036) — tuiles 512 px, `E{level}/{col}_{row}`, même origine et même RLE de présence. Sortie `data/map/colormap/`, manifeste `data/map/colormap.json`. Étages de ~180 m/px à ~20 m/px (le plus fin retenu selon la taille disque ; cible ≤ 400 Mo compressés, format BC1/BC7 ou JPEG selon ce que charge déjà la pyramide de relief).
- **Style** : `data/map/colormap_style.yaml` validé par `data/schemas/colormap_style.schema.json` (palettes, largeurs de routes par `type`, densités, graine).
- **Déterminisme** : graine fixe ; même entrée → mêmes octets.

Couches peintes, dans l'ordre :

1. **Base régionale** : couleur par canal de `splat.png` (prairie, cultures, forêt, roche/lande), modulée par sécheresse, altitude, `forest_kind`, `wetlands` ; bruit basse fréquence contre l'aplat.
2. **Patchwork agricole** : parcelles polygonales (Voronoï déformé ou lanières) autour de `settlements_px.json`, `hamlets.json` et du terroir. Forme régionale : openfield en lanières (nord), bocage à haies (ouest), vigne sur coteaux (sud, pente + exposition). Culture tirée par parcelle dans la palette (blé doré, orge, jachère, pré, labour). Densité décroissante avec la distance au village, fondue vers forêt et lande ; jamais sur roche, neige, eau.
3. **Routes** : `roads.geojson`, terre claire, largeur par `type`, débord adouci, ornières ; lisibles jusqu'à l'étage le plus large (largeur minimale en pixels par étage).
4. **Lacs et rives** : depuis `land_mask.png` (eau intérieure), grève sable/galets, bande de roselière, dégradé d'eau peu profonde.
5. **Villes** : auréole de sol battu et de jardins autour des agglomérations (`towns_1340.json`).

## 2. Rendu — `terrain.gdshader`

- Couleur de base = échantillon de la pyramide de couleur (chargement progressif comme le relief).
- Textures 2k existantes : détail en luminance seulement, fondu avec la distance.
- Relief : `shading_relief` et occlusion de `relief_shade_*` réduits ; valeurs dans les paramètres du matériau, ajustées sur captures.
- Parcellaire procédural (`fine_parcels`, l. 415–492) : désactivé au-dessus du palier « détail proche » (150), conservé dessous par-dessus la carte.
- `road_line` : masqué en vue large et moyenne (routes peintes) ; rubans de route proches inchangés.
- Parchemin (> 1200) : inchangé.
- Repli : si la pyramide de couleur est absente, l'ancien chemin reste actif (drapeau de matériau), pour ne pas casser un clone sans cache.

## 3. Lacs

`game/scripts/map/lakes_renderer.gd` : maillage d'eau à partir des contours des lacs (extraits hors ligne dans `data/map/lakes.json` par le même outil), matériau `river_water.gdshader` (reflets, bord doux, profondeur). Roselières peintes dans la carte ; en vue rapprochée, touffes de roseaux en imposteurs (décor ADR 0137). Le masque d'eau plate actuel du shader reste pour les pixels non couverts.

## 4. fal.ai

Budget 3–6 $, consigné dans `docs/budget.md`.
- 4 à 6 textures tuilables de détail : chaume, labour, rangs de vigne, grève, roselière, terre de route.
- Si le rendu proche le demande : 2–3 petits objets 3D (roseaux, meule) passés en imposteurs.

## 5. Tests et vérification

- pytest : déterminisme ; présence d'une route, d'un lac avec rive et de parcelles sur des tuiles témoins ; validation du YAML par le schéma.
- Test headless Godot `game/tests/sl_colormap_test.gd` : chargement des tuiles, compilation du shader, repli sans pyramide.
- Captures `game/tests/sl_shot.gd` : avant/après vue large et vue moyenne (budget 3 captures).
- Performance : temps GPU du terrain ≤ l'actuel (banc de `docs/wip/fps-carte.md`).

## 6. Lots

1. SL1 — squelette : module, schéma, YAML, test pytest désactivé, note `docs/wip/sl.md`.
2. SL2 — couches 1–5 peintes, pyramide générée, tests pytest.
3. SL3 — branchement shader, dosage du relief, repli, test headless.
4. SL4 — lacs (extraction + `lakes_renderer.gd`).
5. SL5 — textures (et éventuels objets) fal.ai.
6. SL6 — captures, banc de performance, ADR `docs/decisions/0139-sol-satellite-pyramide-de-couleur.md` (numéro à confirmer au moment de l'écriture).

## Hors périmètre

Parchemin, arbres, bâtiments, saisons dynamiques de la carte de couleur (la pyramide est une saison moyenne ; `campaign_life` continue de moduler de près).
