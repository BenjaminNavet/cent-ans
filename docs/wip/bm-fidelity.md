# bm-fidelity — écarts de rendu Bevy / Godot

## Campagne (lot campvis, branche bm/campvis)

Méthode : `game/tests/gr_regions_shot.gd` via `tools/godot_bg.sh` (Beauce 1,6 / 48,25 ; distances 2600, 1200, 400, 120, 30 ; été, voile de guerre levé) contre `--shot` Bevy aux mêmes `CENT_ANS_CAMERA="1.6,48.25,<d>"` (`CENT_ANS_WEATHER=clear CENT_ANS_NO_FOG=1 CENT_ANS_SEASON=summer`). Couleur moyenne du sol au centre (R,G,B) : Godot 400 = 106/93/61, Bevy avant = 158/140/66, Bevy après = ~125/110/65 ; écart-type Godot 23, Bevy ~14.

### Corrigé dans ce lot
- Terrain « absent » de près : cause racine double. (1) La caméra d'interface (MSAA 4 par défaut) n'avait pas le MSAA de la caméra de campagne (Off) : en `--offscreen` elle recouvrait tout le monde 3D (même famille que le correctif de bataille ; `campaign/camera/mod.rs`, hors dossier, minimal). (2) Le test de frustum CPU de Bevy écartait des nœuds entiers du LOD (trous sur le plan « abîme » brun, de près, fenêtré aussi) : `NoFrustumCulling` sur les nœuds, la sélection CDLOD fait déjà le tri (`terrain/lod.rs`).
- Arbres lointains en bruit gris : la sortie de l'imposteur gardait un alpha < 1 après l'éclairage (`billboard.wesl`, alpha forcé à 1) ; les modèles glb décimés (`lod2`, nuées de feuilles sans teinte saisonnière) sont coupés par défaut (`CENT_ANS_VEG_MODELS=1` pour les voir ; Godot lit `lod1` + `foliage_model.gdshader`).
- Lumière de campagne (`terrain/lighting.rs`, port de `campaign_lighting.gd` + `turn_light.gd` + `atmosphere.json campaign.seasons`) : soleil de saison (18-21°, azimut 245), variation d'un tour à l'autre, ambiance froide, crépuscule pendant la résolution du tour (alimente `SkyDusk`), étalonnage de l'environnement (saturation 0,77, contraste, lift, hautes lumières) appliqué dans `terrain.wesl`, exposition 0,78 (équivalent du filmic de Godot).
- Canopée des forêts (`canopy_field` de `terrain.gdshader`) : relief et contraste de teinte.
- `ReliefState` ne se déclare plus modifié à chaque image (écriture seulement au changement réel).
- Départ de campagne cadré sur la capitale du joueur (`campaign/camera/mod.rs`, hors dossier) sauf `CENT_ANS_CAMERA`.
- Plan des villes (ordinaires et emblématiques) sur fil de travail (`towns/layer.rs`, `layer/landmark.rs`) : 30-40 ms hors du fil principal (dump `campaign_towns.plan_ms_max` = durée sur le fil).

### Écarts restants, par impact visuel
1. **Sol de près (400-30)** : champs trop lisses. Godot a des parcelles aux matières propres (`tx_campaign_parcels`), des défrichements de terroir (`terroir_mask`), des saisons par culture (`season_field`), une variation macro GA4 ; le sol Bevy n'a que des aplats et un grain (écart-type 14 contre 23). Fichiers : `terrain.gdshader`, `satellite_ground.gdshaderinc`, `hb_ground.gdshaderinc`.
2. **Forêts** (lot vegvis) : imposteurs plats et sombres, pas de modèles 3D (`foliage_model.gdshader`, teinte saisonnière, `lod1`), pas d'ombres portées, forêt dense à 30-120 absente ; en parchemin, les massifs noirs à l'encre de Godot manquent.
3. **Rivières invisibles à 1200-120** (lot water) : étiquettes « Loire », « Seine » présentes mais pas de ruban bleu, alors que Godot les montre à toutes distances.
4. **Frontières épaisses et hachures d'occupation** (`faction_borders.gdshaderinc`, dans `terrain.gdshader` et `terrain_parchment.gdshader`) : absentes du shader de terrain Bevy (seuls les tracés drapés de `provinces/` existent).
5. **Ombres portées** : aucune (relief, arbres, villes, armées) ; Godot en a (soleil bas) ; le relief n'ombre pas non plus le sol (seulement l'ombrage par normale).
6. **Brume** (lot weather) : `density_for = 0,5 / distance` rend la brume très lourde de près les jours de brouillard et la brume du matin de début de tour (voile blanc à 30-120) ; Godot dose la perspective aérienne selon l'inclinaison (`campaign_lighting.json aerial`, `map_atmo_fog.gdshader`).
7. **Caméra** : à distance égale la vue Bevy couvre plus de terrain que Godot (inclinaison ; `close_camera.tres`, `campaign_camera.gd`), à vérifier côté lot camera.
8. **Étalonnage des objets** : l'étalonnage n'est appliqué qu'au terrain ; mer, rubans, maquettes, arbres restent plus saturés (utiliser `ColorGrading` de la caméra pour les PBR, et le shader mer).
9. **Mode Politique** à 1200 : lavis pastel déjà à demi-poids (`strategic = 0,5`) alors que Godot garde le sol naturel et ne colore que les frontières.
10. Modèles glb décimés : texture saisonnière et `model_chroma` (`tree_model_gains.json`) à porter dans un matériau dédié (le pipeline instancié de bataille est partagé).
