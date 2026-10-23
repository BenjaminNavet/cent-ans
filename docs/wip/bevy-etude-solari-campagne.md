# Étude : Solari en campagne (lecture seule, 22/10)

Aucune compilation ni lancement. [E] = établi par lecture du code, [S] = supposé / à mesurer.

## 1. Inventaire du rendu de campagne

Caméra (`campaign/camera/mod.rs` ~272-290) [E] : `Camera3d`, near 1, far 6000, `Tonemapping::AgX`, MSAA réglé par
`settings::quality` (`msaa_campaign`), pas de Hdr explicite, aucun prepass (`enable_prepass() = false` sur le terrain). Une passe
post maison après tonemap (`terrain/post.rs`) lit profondeur et vue, recompose brume de vallée, perspective aérienne, froid.
Soleil : une seule `DirectionalLight` `CampaignSun` (`terrain/mod.rs:589`), cascades dimensionnées par `terrain/shadows.rs`
(coupées au-delà de `max_camera_distance` 650, `data/fx/campaign_lighting.json`). Pas de cycle jour/nuit (design
`carte-vivante-folk.md` : « Pas de cycle jour-nuit ») ; la lumière varie par saison/tour (`terrain/lighting.rs` : élévation,
azimut, couleur) avec un crépuscule doré en fin de tour.

| Famille | Mesh ? | Matériau | Éclairage | Compat Solari |
|---|---|---|---|---|
| Terrain (`terrain/`) | Oui : 1 entité + 1 `Mesh3d` par nœud CDLOD, maillage CPU (`lod.rs`, `mesh.rs`), patch 64x64 quads + jupe, ~8,7 k triangles/nœud, 350-900 nœuds (donc ~3 à 7,5 M triangles) | `TerrainMaterial` (`Material` pur, pas StandardMaterial), `terrain.wesl` 2401 lignes | Forward maison : `ambient + sun*n.l*sun_vis`, ombre = `fetch_directional_shadow` (cascades), plancher d'ombre, ombres de cumulus, brouillard, grade ; relief déplacé au vertex (`displayed_y` : exagération verticale dynamique, géomorphe, enfoncement des bords) | Incompatible tel quel : matériau non Standard, géométrie du BLAS différente de la géométrie affichée (déplacement vertex), sortie non G-buffer |
| Eau (`water/`) : mer, rivières, lacs, routes, lignes | Oui (rubans/polygones drapés) | `SeaMaterial`, `RiverMaterial`, `FreshMaterial`, `LineMaterial`, `RoadRibbonMaterial` : `Material` purs | Forward maison | Incompatible ; reste en forward, hors TLAS |
| Végétation (`vegetation/`) | Non pour les arbres : batches instanciés `Transparent3d` à shader perso `billboard.wesl` (cartes face caméra) ; ombres via passe non-mesh `shadow_cast.wesl` (`shadow_pass.rs`, `BinnedRenderPhaseType::NonMesh`) | Pipeline maison, pas un `Material` | PBR Bevy appelé depuis le shader | Hors TLAS par construction (pas de `Mesh3d`) ; la passe d'ombre non-mesh ne servirait plus (Solari remplace les shadow maps) |
| Armées (`armies/`) | Oui, skinned (`skinned.rs`), plus drapeaux (`FlagMaterial`), voiles (`SailExt`), bulles | Mix `StandardMaterial` / extensions | Forward Bevy | Skinned : BLAS statique, donc substituts rigides ou exclus [S] |
| Villes (`towns/`) | Oui, kits, atlas, `StandardMaterial` pour rues/sol (`towns/layer.rs`) ; échelle `s = 1/719` | Mix | Forward | Géométrie minuscule face à `RAY_T_MIN = 0.001` (cf. 2) |
| Marqueurs, vie, campagne (`markers/`, `life/`, `countryside/`) | Oui, petits meshes ; extensions (`BannerExt`, `KitExt`, `AnimExtension`) | `ExtendedMaterial` | Forward | Extensions exclues de la TLAS (comme en bataille) |
| Parchemin/mer stylisés (`parchment/`) | Oui, plans | `ParchmentSeaMaterial` etc. | Non éclairé (émission 0,75) | Sans objet (mode stylisé) |
| Météo (`weather/`) : sprites, `DistanceFog`, nuages, cumulus | Sprites/shaders | Maison | Pas de matière | Hors scène RT ; l'ombre des cumulus est un terme du terrain shader, à réécrire dans `base_color` si le terrain passe en G-buffer |
| Brouillard de guerre, post-traitements | Passe plein écran / terrain | Maison (`post.wesl`) | Après tonemap | Compatible (après Solari), mais la passe lie la profondeur en variante multisample : à revoir |

Prérequis Solari rappelés (`docs/wip/bevy-solari.md`) [E] : seules les entités `RaytracingMesh3d` + `MeshMaterial3d<StandardMaterial>`
entrent dans la TLAS ; `SolariLightingPlugin` impose le rendu différé par défaut à toute l'application ; caméra : Hdr,
Msaa::Off, DeferredPrepass+DepthPrepass+MotionVectorPrepass. Conséquence : tout matériau forward maison non converti doit
être forcé en forward (`force_forward`, comme `battle/scene/solari.rs`) et reste non éclairé par Solari. En campagne
cela concerne presque tout.

## 2. Faisabilité technique à l'échelle

- **Géométrie du terrain** [E] : le BLAS devrait contenir la géométrie déplacée, or le déplacement (exagération verticale
  réglable, géomorphe, enfoncement des bords) se fait au vertex shader à partir d'attributs (altitude m, champs). Il faudrait
  une copie CPU « cuite » par nœud (position finale), refaite à chaque changement d'exagération/saison.
- **Coût BLAS** [S] : 350-900 nœuds, 3 à 7 M triangles contre `max_triangles` 4 M du prototype bataille. Les nœuds entrent et
  sortent quand la caméra bouge (nœuds « retiring » jusqu'à 2 s, `lod.rs`), donc des dizaines de constructions de BLAS par
  seconde en déplacement ; Solari construit en mode `Build` (pas de refit) puis compacte (400 k sommets/frame max,
  `scene/blas.rs`). Les slabs de mesh sont limités à 500 (`MAX_MESH_SLAB_COUNT`, `binder/instances.rs`) [E] : plus de 900 nœuds
  pose question [S].
- **Précision** [E] : coordonnées jusqu'à 7080, f32 ~0,5 mm, acceptable ; mais `RAY_T_MIN = 0.001` en unités de carte
  (`scene/bindings.wesl:101`) : une ville à `s = 1/719` (bâtiment de 10 m = 0,014 unité) est de l'ordre de l'epsilon, donc
  auto-ombrage et fuites de lumière probables. Le RT ne profite qu'au relief. `RAY_T_MAX` 100 000 couvre la carte.
- **Caméra** [S] : zoom d = 20-150 ; en vue haute le RT ne rend plus que soleil rasant et AO de vallée.
- **Lumières** [E] : une `DirectionalLight` seule est une source valide (`binder/lights.rs`, `GpuDirectionalLight`), échantillonnée
  par next-event estimation ; les émissifs (fenêtres de villes, feux) sont des lumières par triangle (max 65 535 triangles par
  mesh et 65 535 sources). Le ciel froid maison (ambient du shader) devrait passer en environment map ou ambient global.
- **Débruitage** [E] : seul DLSS-RR (NVIDIA) ; sur Apple, bruit résiduel (ReSTIR/world cache seulement), gênant sur de grands
  sols lisses.
- **Saisons et neige** [E] : neige, boue, splat 6 couches, ombres de cumulus, teinte saisonnière sont calculés dans `terrain.wesl`.
  Le G-buffer ne porte qu'un albedo + rugosité + réflectance mono + occlusion mono : le terrain devrait écrire un `base_color`
  final (via `deferred_output`) et perdre grade/brume/brouillard de guerre intégrés (à refaire en post).
- **Limites vues en bataille** : Metal plafonne à 16 samplers (plantage actuel de la branche, features pbr, voir
  `bevy-solari.md` § points ouverts) ; sol de bataille 13 k triangles (pas 12 m), contre 3-7 M ici.
- **Caméra** [E] : MSAA campagne à couper, HDR requis ; le look (AgX + grade post tonemap) est à revalider.

## 3. Valeur visuelle

Carte vue de haut, soleil de saison à 14-34° d'élévation, relief modeste en unités carte.
- Ce que Solari apporte : ombres portées de relief de toute portée (au-delà des cascades), occlusion ambiante de vallée, rebonds
  sur pentes ensoleillées, ombres de villes/armées sur le sol. À d = 100+, l'essentiel du visible est ombre de relief rasante +
  AO de vallée.
- Déjà fait sans RT [E] : ombres de cascade sur le relief (`terrain_shadow.wesl` + `sun_shadow`), plancher d'ombre ciel (TS-T9),
  AO `rvd.ao`, brume de vallée, perspective aérienne, ombres de cumulus.
- Alternatives sans RT :
  1. **Ombre de relief par ray-marching du heightfield dans `terrain.wesl`** (soleil unique, direction connue par saison) :
     16-32 pas sur les pages de relief déjà en GPU (`relief_gpu.rs`, `relief.wesl`), ombres exactes sur toute portée, y compris
     au-delà de 650 où les cascades sont coupées. Petit coût, compatible neige/saisons/brouillard/Mac.
  2. **Horizon mapping / AO de pente précalculé** (champ dans les pages générées par `tools/`) : quasi gratuit au runtime,
     donne AO de vallée et pénombres.
  3. SSAO Bevy : exige prepass profondeur+normales, MSAA off ; faible gain en vue haute.
- **Verdict** : 1 + 2 donnent ~80 % du résultat de Solari sur le terrain à ~20 % (ou moins) du coût, sans réécrire les matériaux.
  Solari ne gagne vraiment que sur les rebonds couleur et les ombres d'objets (villes, armées, arbres), peu lisibles à cette
  échelle. Honnêtement, Solari en campagne reste un effet vitrine coûteux, bruité sans débruiteur Apple.

## 4. Plan en lots

Ordre recommandé : A, B (sans RT) avant tout RT ; C-E seulement si l'utilisateur tient au RT.

| Lot | Contenu | Effort | Risque |
|---|---|---|---|
| A. Ombre de relief ray-marchée | `terrain.wesl` : marche d'ombre sur le heightfield, `sun_vis = min(cascade, marche)`, réglages dans `campaign_lighting.json`, désactivable | petit à moyen | précision sur pages fines, bords de pages |
| B. AO d'horizon / pente précalculé | champ AO dans les pages (génération `tools/`) ou calculé au chargement, lu dans `rvd.ao` | moyen | pipeline de données |
| C. Banc RT terrain seul (prototype) | module `campaign/solari.rs` derrière la feature `solari` : terrain simplifié en `StandardMaterial` dans un BLAS grossier (~500 k triangles, géométrie déplacée cuite CPU), caméra Hdr/Msaa off/prepass, soleil seul ; le reste forcé en forward. Mesure : `tools/bevy_bench/bench.sh solari_camp --offscreen` contre une base (ms GPU, mémoire BLAS, coût de reconstruction au déplacement) à d = 20, 60, 150 | moyen | `MAX_MESH_SLAB_COUNT` 500, 16 samplers Metal, bruit sans débruiteur |
| D. Terrain différé réel | `deferred_fragment_shader` pour `TerrainMaterial` (G-buffer), brume/grade/cumulus en post ou dans `base_color`, BLAS par nœud via substituts mis à jour avec le quadtree | gros | refonte de 2400 lignes, régressions de look, coût BLAS en mouvement |
| E. Objets (villes, armées, arbres) | substituts par famille comme en bataille ; villes `s = 1/719` contre `RAY_T_MIN` à contourner (échelle de BLAS locale) | gros | epsilon, skinned, budget d'instances |

Prototype minimal mesurable : lot C seul. Critère d'arrêt proposé (hypothèse) : passe Solari < 4 ms sur M4 Pro et reconstruction de
BLAS lissée en mouvement ; sinon on s'en tient à A+B. Préalables : aucun banc concurrent (`pgrep -f "bench.sh perf_"`) et
plantage Metal des 16 samplers résolu (lot pbr).

## Points ouverts à trancher par l'utilisateur
- A+B (rapide, sans RT) comme réponse principale, Solari restant mode expérimental ?
- Si RT voulu : accepte-t-il que seul le relief y entre au début (villes/arbres/armées exclus) ?
- Bruit sans débruiteur sur Apple : acceptable ?
