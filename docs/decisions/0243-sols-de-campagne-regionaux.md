# 0243 — Sols de campagne régionaux

## Contexte
TX 2b : les 7 couches globales GA4 du fond de campagne (herbe, culture, forêt, roche, lande, neige,
sable) ne distinguent pas les régions. Les paquets `tx_campaign_bg` (45 couches), `tx_campaign_parcels`
(42) et `tx_micro_ground` (8 grains) sont générés (ADR 0236) ; les 14 biomes existent (ADR 0242).

## Décision
- **Fond par biome** : `data/fx/campaign_terrain_textures.json` gagne un bloc `regional` (table rôle du
  shader -> rôle du paquet, surcharges par biome, ex. lande = sous-bois en milieu atlantique/boréal).
  `CampaignTextures.build_layer_table` en tire `bg_layers[biome * 7 + rôle]` (repli sur le parent) ;
  `terrain.gdshader` choisit la couche par rôle selon le biome du pixel. Neige et sable sont partagés.
  La couleur reste celle de la carte de couleur (teintes par rôle) ; le fond apporte le grain, la
  structure et une part de teinte propre (`hue_keep`, relative à la moyenne des couches du même rôle).
  Les champs cultivés réutilisent l'herbe/sol nu du biome (pas de couche de labour dans ce paquet).
- **Fondu de frontière** : `cent-ans geo biome-blend` cuit deux rasters demi-résolution
  (`biomes_blend_ab.png` : biome et voisin le plus proche ; `biomes_blend_dist.png` : distance au
  voisin). Le poids du voisin vaut 0,5 à la frontière et décroît jusqu'à 0 à `blend_km` (10 km) : les
  deux côtés concordent, aucune couture. Le second biome n'est lu que sous le seuil `min_blend_weight`
  (coût : 2 lectures de la carte, tuilage supplémentaire seulement près des frontières).
- **Micro-détail** : paquet `tx_micro_ground`, un grain par rôle, fondu en luminance de près
  (`fade_footprint`), `micro_on` faux sans lui.
- **Parcellaire** : drapeau `parcels_source` (`tx` par défaut, `hb`) dans `ground_biome_mix.json` ;
  `tx_overrides` donne les matières régionales (oasis, dunes, toundra...) quand la source est `tx`.
  `GroundMaterials.source` choisit le manifeste ; le plan des champs DN (`FieldPlan`) lit le mélange
  d'origine, non modifié.
- **`--legacy-textures`** : ancien rendu exact (couches GA4, parcellaire HB). Réglage
  « Qualité des textures : Haute / Moyenne » (`video/texture_quality`) : « haute » prend les paquets 2k
  `hi/` + manifeste `_2048` s'ils existent ; appliqué au prochain chargement de la carte.

## Conséquences
- `layer_mean` passe à 48 entrées ; `sample_layer` utilise `textureGrad` (dérivées hors branches).
- Mémoire : 45 couches 1k (≈ 15 Mo VRAM compressées) ou 2k (≈ 250 Mo).
- Défaut `parcels_source = tx` (planche `fields_compare.png`, 2026-10-09) : les trois vues sont quasi
  identiques, `tx` ajoute les matières régionales absentes de `hb` ; champs 3D DN inchangés.
  `hue_keep` relevé à 0,9 (fond régional trop discret à 0,6).
- ADR 0244 : les 7 couches GA4 globales Poly Haven, leur chemin de repli 1k et `--legacy-textures` pour le sol de campagne sont retirés ; le bloc `regional` est la seule voie (repli parent puis couche 0).
