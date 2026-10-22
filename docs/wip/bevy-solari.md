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
