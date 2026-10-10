# ZF-C : LOD brutaux au zoom (arbres, villes)

État (10-10) : cibles 1 à 4 codées, vérification Godot en cours.

- Cible 1 (`vegetation.gd` `_apply_lod`) : avec les imposteurs GA3 (défaut), le palier DETAILED partage le maillage des imposteurs : aucun échange à `detail_distance`. Ajouté : hystérésis `LOD_HYSTERESIS` (8 %) sur le palier. Ombres : toujours par partie (pas de fondu possible sans passe dédiée).
- Cible 2 (`model_zone`) : poids par arbre `model_zone_weight` (`foliage_common.gdshaderinc`), bande `generalised_model_fade` (`map_prop_scale.gd`, 0,35 du rayon) ; imposteurs et modèles tramés en complément (Bayer, `bayer4` déplacé dans `foliage_common`).
- Cible 3 (`forest_detail.gd`) : une partie à moins de `Vegetation.cards_reach()` + `near_margin` de la caméra garde le maillage à cartes (fondu par arbre déjà dans le shader).
- Cible 4 (villes) : `TownRenderProfile.layer_weight` (bande `layer_fade_band` 0,15 de `max_rig_distance`) ; `layer_weight` dans `town_building.gdshader` (keep) et `town_far.gdshader` (enfoncement).

Prochaine étape : smoke + tests ; contrôle visuel par la session principale.
