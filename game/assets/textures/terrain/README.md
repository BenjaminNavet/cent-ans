# Textures du terrain de campagne et de la mer

Le sol de campagne ne vient plus de Poly Haven (ADR 0244) : le fond régional TX (paquet
`tx_campaign_bg_*`, ADR 0243), le grain de sol (`tx_micro_ground_*`) et le parcellaire
(`tx_campaign_parcels_*`, `hb_ground_*`) sont générés ; leurs manifestes sont dans `data/art/`.

- `water_normal.png` : normale de mer tuilable 1024², **procédurale** (œuvre propre, CC0), somme de
  trains d'ondes à vecteurs d'onde entiers (raccord exact), direction de vent dominante
  (`tools/cent_ans_tools/geo/textures.py`, `uv run --project tools cent-ans geo textures`).
  Importée en carte de normales (RGTC).

Réglages (macro-variation, eau, bloc `regional`, `micro`) : `data/fx/campaign_terrain_textures.json`
(schéma `fx_campaign_terrain_textures.schema.json`).
