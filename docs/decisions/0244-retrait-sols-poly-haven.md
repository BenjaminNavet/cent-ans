# 0244 — Retrait des sols Poly Haven (bataille et campagne)

Statut : accepté (2026-10-09)

## Contexte
Les paquets TX régionaux (ADR 0240 bataille, 0243 campagne) sont le rendu par défaut du sol. Les
anciennes textures Poly Haven restaient comme repli (`--legacy-textures`, paquet manquant) :
132 Mo côté bataille, 120 Mo côté campagne, des schémas, des outils de téléchargement et des
crédits. Le joueur a donné son feu vert sans attendre de partie pilote.

## Décision
- Bataille : retrait du bloc `legacy` de `data/fx/battle_ground_layers.json` (`poly_haven_id`,
  `terrain_tints`), des tableaux `ground_*_array.jpg` et du détail proche `near_detail/`. Repli de
  `BattleGroundTextures.pack_for` : biome, parent, `default_biome` (paquet TX), jamais Poly Haven.
  `BattleTerrain.ground_layers()` dérive des `roles` ; `terrain_tint` disparaît (les biomes
  steppe/désert portent leur couleur). Le grain TX `micro_battle` (fondu sous 32 m, une couche par
  rôle de sol) couvre le rôle du détail proche : ses uniformes `near_detail_*` et sa branche de
  shader sont supprimés, `micro_dist` reprend la distance de fondu.
- Campagne : retrait des 7 couches GA4 globales (`layers`, `albedo_size`, `normal_size` des données,
  fichiers `terrain/<couche>_*.jpg` et `terrain_*_array.jpg`, chemin de repli 1k de
  `TerrainBuilder`, `CampaignTextures.load_arrays/layer_ids`). Le bloc `regional` est la seule
  voie ; repli parent puis couche 0 dans `build_layer_table`. Sans paquet, terrain sans texture
  (avertissement), plus de chemin Poly Haven.
- Outils : `geo/textures.py` ne produit plus que la normale de mer procédurale ; `cent-ans geo
  textures` n'a plus d'option `--force`. Schémas, tests, `CREDITS.md` et README mis à jour.
- `--legacy-textures` / `TextureQuality.use_tx()` n'affecte plus le sol ; il garde le parcellaire
  HB, la couche par défaut des bâtiments, la végétation, l'eau et les écorces.

## Conséquences
- Environ 250 Mo de textures en moins dans le dépôt (historique inchangé).
- Gardés car encore utilisés par le rendu par défaut : ciels HDRI (`atmosphere.json`,
  `atmosphere_library.gd`, `battle_atmosphere.gd`), matières de siège (`battle_siege.gd`),
  couche par défaut des bâtiments (`building_materials.json`), murailles/toits/écorce de
  `textures/battle/` (hors sol).
- Si les paquets TX d'un biome sont régénérés, seuls leurs manifestes `data/art/tx_*_pack.json`
  comptent ; aucun repli visuel de secours n'existe.
