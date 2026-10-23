# bevy_solari (lot solari)

État : prototype en place derrière la feature Cargo `solari` (désactivée par défaut) et le réglage
`data/fx/solari.json` (`enabled: false`, schéma `data/schemas/fx_solari.schema.json`). Code :
`client/src/battle/scene/solari.rs`, branché dans `client/src/main.rs`.

## Prérequis (lecture de bevy_solari 0.20, wgpu 30.0.1)

- Features wgpu exigées (`SolariPlugins::required_wgpu_features`) : `EXPERIMENTAL_RAY_QUERY`,
  `BUFFER_BINDING_ARRAY`, `TEXTURE_BINDING_ARRAY`, `SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING`,
  `PARTIALLY_BOUND_BINDING_ARRAY`.
- Metal (wgpu-hal 30.0.1, `metal/adapter.rs`) : `EXPERIMENTAL_RAY_QUERY` si macOS >= 15 et
  `device.supportsRaytracing()` + `supportsRaytracingFromRender` ; les binding arrays demandent MSL >= 3.0
  et argument buffers Tier 2. Un M4 Pro (famille Apple 9) y répond en principe : à confirmer au runtime (ci-dessous).
- Matériaux : seules les entités `RaytracingMesh3d` + `MeshMaterial3d<StandardMaterial>` entrent dans la TLAS
  (`scene/binder/instances.rs` interroge ce type exact). `ExtendedMaterial` (notre sol `GroundMaterial`),
  soldats instanciés et plantes (matériaux custom) en sont exclus : ni ombres RT ni GI pour eux.
  Les meshes doivent avoir UV0 + tangentes + indices u32 (corrigé dans `trace_standard_meshes`).
- `SolariLightingPlugin::build` impose `DefaultOpaqueRendererMethod::deferred()` à toute l'application,
  même sans GPU compatible : d'où la sonde d'adaptateur AVANT d'ajouter le plugin.
- Caméra : `Msaa::Off`, `CameraMainTextureUsages` + `STORAGE_BINDING`, `Hdr`, prepass profondeur + deferred +
  vecteurs de mouvement (via `#[require]`). Les ombres de lumière sont remplacées (shadow_maps_enabled false).
- Débruiteur : seul DLSS Ray Reconstruction (NVIDIA, features `dlss`) ; aucun débruiteur sur Apple, ReSTIR optionnel.

## Comportement

`solari.json.enabled = true` + build `--features solari` : sonde wgpu (HighPerformance, comme Bevy) ; si les
features manquent ou pas d'adaptateur : une ligne `info!` « solari: GPU lacks ... » et retour au raster, sinon
`SolariPlugins` ajouté et la caméra de bataille reçoit `SolariLighting`.

## Prochaine étape

Vérification clippy/tests, essai runtime via `client/run_bg.sh` (CENT_ANS_MUTE=1, --offscreen) pour lire la ligne
de sonde, puis mesure au banc (aucun banc ne doit tourner).

## Point d'arrêt (22/10)

- `cargo clippy --features dev,solari --all-targets -D warnings` passe (sans warning). `cargo fmt` à repasser.
- Le lancement runtime (sonde réelle sur M4 Pro) et `cargo test` n'ont PAS pu tourner : disque plein
  (284 Mo libres sur 926 Gio, `No space left on device` pendant la compilation debug). À reprendre quand de la
  place est libérée : mettre `"enabled": true` dans data/fx/solari.json, puis
  `CENT_ANS_MUTE=1 CENT_ANS_FEATURES=dev,solari CENT_ANS_PROFILE=debug client/run_bg.sh --state Battle --frames 200 --offscreen`
  et lire la ligne `solari: ...` du log ; remettre `false` avant commit.
- Mesure au banc : non faite (même raison). Attendu : sol, soldats et plantes hors TLAS (matériaux custom), donc
  gain visuel limité tant qu'ils ne sont pas portés vers `StandardMaterial` ; pas de débruiteur hors NVIDIA.

## solari2 (22/10) : plantage « Mesh has been extracted » et entités de substitution

Cause : `RaytracingMesh3d` partageait le `Handle<Mesh>` du rendu raster ; ces meshes n'ont pas `MAIN_WORLD`
dans leurs `RenderAssetUsages`, leurs données quittent le monde principal après extraction, et le système
`trace_standard_meshes` (accès aux attributs) paniquait. Ce système est supprimé.

Correction (`client/src/battle/scene/solari.rs`) : entités de SUBSTITUTION.
- Décor statique : pour chaque entité `Mesh3d` + `MeshMaterial3d<M>` avec `M` = `StandardMaterial`,
  `SiegeStoneMaterial`, `BuildingAtlasMaterial`, `WindMaterial` (écorces/feuillages d'arbres), un ENFANT (même
  transform, sans `Mesh3d`, donc invisible au raster) porte `RaytracingMesh3d(copie réduite)` +
  `MeshMaterial3d<StandardMaterial>` de la couleur moyenne (couleur de base x moyenne des couleurs de sommet).
  La copie ne garde que position/normale/UV0/tangente + indices u32 (format exigé par `is_mesh_raytracing_compatible`) avec
  `MAIN_WORLD|RENDER_WORLD`. Cache par mesh source : les instances partagent le même BLAS. Les meshes déjà
  sortis du monde principal (lecture `try_*` en erreur) sont ignorés (cachés comme `None`).
- Les LOD lointains (`VisibilityRange` dont `start_margin.end > 0`) sont ignorés ; les soldats instanciés n'ont pas de
  `Mesh3d` par entité et sont exclus.
- Sol : grille grossière (`ground_step_m`, 12 m) construite depuis `Battlefield`, couleur de saison, entité séparée.
- Budgets dans `data/fx/solari.json` (`max_triangles` 4 M, `max_instances` 40 000, `ground_step_m`) ; le total est
  journalisé (`solari: N substitute instances, M distinct meshes, T triangles, S skipped`).

### Piège signalé par l'étude vent : feuillage invisible

`SolariLightingPlugin` impose `DefaultOpaqueRendererMethod::deferred()` ; un matériau résolu en différé est écarté
de la passe principale (bevy_pbr material.rs ~1344-1395 : `update_cache(.., None); continue`), alpha-mask compris. Nos
matériaux à shader forward perso (ground, wind, stone, atlas) n'écrivent pas dans le G-buffer, donc seraient
INVISIBLES. Correctif appliqué : `force_forward::<M>` met `base.opaque_render_method = Forward` sur ces quatre types
(sur les évènements `AssetEvent::Added/Modified`). Conséquence : ils restent visibles, éclairés par le raster (lumières
sans ombres portées puisque `shadow_maps_enabled` est coupé), non éclairés par Solari ; ils servent seulement
de géométrie aux rayons via les substituts. Résultat runtime : voir « Résultats » ci-dessous.

### Ce qu'il faudrait pour que sol et végétation soient éclairés par Solari (non fait)

Lecture : bevy_pbr 0.20 `deferred/functions.wesl` (`deferred_gbuffer_from_pbr_input`, `deferred_output`),
exemples `shader/extended_material.rs` (`deferred_fragment_shader`) et `shader_advanced/deferred_raymarch.rs`.
1. Retirer `force_forward` et laisser le défaut différé.
2. Pour chaque `MaterialExtension` (GroundExtension, WindExtension, StoneExt, BuildingAtlasExt) : implémenter
   `deferred_fragment_shader()` ; en WESL, construire le `PbrInput` (`pbr_input_from_standard_material`, puis
   modifier `pbr_input.material.base_color` avec le splat/texture/teinte, normale, rugosité) et finir par
   `deferred_output(in, pbr_input)` (au lieu de `apply_pbr_lighting` + `main_pass_post_lighting_processing` de
   ground.wesl). Le fragment forward doit rester pour le chemin forward (`#ifdef PREPASS_PIPELINE` comme dans
   l'exemple Bevy).
3. Végétation (WindMaterial) : `enable_prepass()` est faux ; le passage différé passe par la prepass, donc il faut
   `enable_prepass() = true` et fournir `prepass_vertex_shader()`/`deferred_vertex_shader()` qui appliquent le MÊME
   déplacement de vent que le vertex forward (sinon feuillage figé et vecteurs de mouvement faux), plus
   `prepass_fragment_shader()` pour l'alpha-mask (discard des feuilles) ; les impostors à `enable_prepass` faux restent
   en forward ou doivent suivre le même schéma.
4. Le G-buffer est compact : un seul réflectance monochrome, occlusion monochrome ; les effets du sol faits à la main
   (neige, boue, humidité, splat 6 couches, brouillard de hauteur) doivent être ramenés dans `base_color`/rugosité/
   émissif ; le brouillard et le grade restent post-traitement.
5. Mesurer : le coût différé + RT sur milliers d'arbres (un BLAS par mesh de couronne) est le risque majeur.

### Résultats (solari2, M4 Pro, build debug)

- Plantage « Mesh has been extracted » : CORRIGÉ. `CENT_ANS_FEATURES=dev,solari`, `--state Battle --frames 400 --offscreen` :
  400 images, sortie 0, dump écrit, aucune panique, sonde « ray-query GPU found ».
- Substituts (bataille d'essai) : sol 13 400 triangles (6 868 sommets, pas 12 m) ; décor ~24 800 instances,
  94 meshes distincts, ~94 500 triangles de substitution (0 ignoré par budget). Soldats exclus.
- Conflit B0002 trouvé et corrigé en route (`Assets<StandardMaterial>` lu et écrit dans le même système).
- Visibilité du feuillage avec `force_forward` : NON vérifiée visuellement (aucune capture permise) ; à contrôler par la session principale.

### Points ouverts hors périmètre Solari (à traiter par la session principale)

1. **Plantage Metal pré-existant sur la branche perf/bevy-par-iter, avec ou sans Solari** (testé `enabled:false`) :
   « Too many bindings of type Samplers ... limit is 16, count was 19 » (GroundExtension 19, StoneExt 17,
   BuildingAtlasExt) puis arrêt « Validation RenderError ». Cause : les features Cargo du lot pbr
   (`pbr_anisotropy_texture`, `pbr_multi_layer_material_textures`, `pbr_specular_textures`, `pbr_transmission_textures`)
   ajoutent des samplers au `StandardMaterial` de base ; Metal plafonne à 16. Vérifié : sans ces quatre
   features (en gardant `experimental_pbr_pcss`, utilisé par le code), le client tourne. Je n'ai pas commité ce
   changement de `client/Cargo.toml` (hors périmètre, lot pbr).
2. `ground.wesl` : « module not found: bevy_pbr::render::decal::clustered » quand `battle_decals.json` est `enabled:true`
   sans la feature Cargo `decals` (warning shader, pas de plantage dans l'essai).
3. `enabled: true` est la valeur laissée dans `data/fx/solari.json` (reçue du commit f6f870711) ; sans effet hors build `--features solari`.
4. Mesure fps au banc release : non faite (consigne).

## solari-noir (22/10) : monde noir en bataille, CORRIGÉ

Cause : la texture principale d'une vue est mise en cache par clé (cible, format, USAGES, msaa) (bevy_render
`prepare_view_targets`/`main_texture_key`). La caméra de bataille recevait `STORAGE_BINDING` (+ Hdr, Msaa::Off) pour Solari,
pas la caméra UI (HUD, order 0, sans effacement) : deux textures principales distinctes, le HUD recouvrait la cible avec
sa propre texture (monde absent, fond (19,14,10)). Le même effet existait pour Hdr (déjà géré pour le bloom) et msaa.
Correctif (`battle/scene/camera.rs`) : si Solari est actif, la caméra UI prend Hdr, Msaa::Off et
`CameraMainTextureUsages + STORAGE_BINDING` pendant la bataille (rétablis à la sortie). `lighting.rs` : pas de SSAO/SMAA
avec Solari. Vérifié par capture (monde, ciel, soldats visibles).
Reste : bandes noires sur le sol (auto-ombrage par le sol grossier de substitution, pas de biais) ; pas d'EnvironmentMapLight
(ciel non éclairant) ; `enabled` laissé à false dans solari.json.

## Bancs de bataille release, Solari off/on (23/10, machine calme, une passe chacun)
`tools/bevy_bench/bench.sh solari_off|solari_on --bench-battle --offscreen`, build par défaut (decals + solari compilés), M4 Pro.
- Solari coupé : calme 68,8 fps (p50 14,5 ms) ; mêlée 41,4 fps (p50 24,1 / p99 26,1 ms) ; 65 779 entités max.
- Solari activé : calme 57,6 fps (p50 17,4 ms) ; mêlée 52,3 fps (p50 18,6 / p99 22,2 ms) ; 95 996 entités max (+30 k substituts).
- Lecture (supposée, non isolée) : le gain en mêlée vient de la coupure des cartes d'ombre du soleil (remplacées par les rayons), pas des rayons. Coût visuel : les soldats (hors TLAS) ne projettent plus d'ombre.
- Décision : `enabled:false` par défaut. Pour trancher : lot 0 foule (variantes CENT_ANS_NO_FIGURE_SHADOWS / CENT_ANS_BATTLE_LOD) puis soldats dans le TLAS ou ombres cascades conservées pour eux seuls.

## Décision Solari en campagne (23/10, après capture)
Le prototype (terrain de substitution 500 k triangles, `campaign/terrain/solari_proxy.rs`, correctif de la caméra UI) fonctionne : carte affichée sans prologue bloqué. Mais le rendu est plus pâle et plus plat que le rendu normal (pas de mélange de sol, de brume ni de saisons ; ombres de relief peu lisibles à cette hauteur ; bruit sans débruiteur). Les bancs précédents (pan 12,4 ms vs 17,2) ont été pris avec le monde recouvert par la caméra UI : à REFAIRE si on rouvre le sujet. Décision : désactivé (`enabled:false`), pas de shader différé de terrain (option (a) de bevy-etude-terrain-std.md) tant qu'une alternative n'a pas été essayée : ombre de relief ray-marchée du heightfield + AO d'horizon (étude solari-campagne, lots A et B).
