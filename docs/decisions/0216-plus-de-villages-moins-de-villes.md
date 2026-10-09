# 0216 — Plus de villages, moins de villes sur la carte de campagne

Date : 2026-10-09

## Contexte

Retour du joueur : « trop de villes, pas assez de villages ». Les données comptent 438 cités et
702 bourgs pour 419 villages ; les 2 988 hameaux de `hamlets.json` n'étaient dessinés que sous le
palier de détail proche (caméra < 150), à taille quasi réelle (≈ 1,8 unité contre 5 pour une
maquette de village), donc invisibles ; les villages disparaissaient dès 400, les bourgs à 900.

## Décision

Affichage seul (le type de règle des lieux ne change pas, `core/` intact) :

- **Rétrogradation d'affichage** (`display_demotion` de `data/art/town_maquettes.json`) : une cité
  de poids ≤ 12 prend la maquette de bourg (117 lieux), un bourg de poids ≤ 8 celle de village
  (180 lieux).
- **Villages visibles jusqu'à 900**, comme les bourgs.
- **Hameaux** visibles jusqu'à `hamlet_range` = 600 (au lieu du seul détail proche) et dessinés
  par le village généré (DN, ADR 0211/0214) de la famille d'architecture de leur province, à
  `hamlet_village_ratio` = 0,7 de la largeur d'un village ; repli sur les hameaux du kit sans glb
  généré ou en style `real`. Préparation des glb DN partagée (`DnCampaignModels.prepare`).

## Conséquences

- La campagne montre des villages entre les bourgs aux zooms moyens et proches.
- Plus d'instances de hameaux construites (tuiles de terrain « proches », < 600) : à surveiller au
  banc sur machine calme.
- Réglages dans les données : seuils de poids, ratio et portées.
