# Audit : ce que le client réimplémente alors que Bevy 0.20 le fait

État : TERMINÉ (lecture seule, aucune compilation). Sources Bevy : `~/.cargo/registry/src/*/bevy_*-0.20.0`.
Remarque : `git merge --ff-only perf/bevy-par-iter` impossible (la branche de ce worktree, issue de main, a 3 commits que `perf/bevy-par-iter` n'a pas ; les deux ont divergé). Audit fait sur l'arbre de ce worktree ; `docs/wip/bevy-misc.md` lu via `git show perf/bevy-par-iter:`. Aucun écart de fond attendu (les lots de la branche perf touchent features/réglages).

## Tableau

| # | Sujet | Notre code | Équivalent Bevy 0.20 | Verdict | Effort / risque |
|---|---|---|---|---|---|
| 1 | Mips des textures glTF | `fx/model_mips.rs` (206 l.) : chaîne box CPU, 2 images/frame, sampler aniso 8 | `bevy_core_pipeline-0.20.0/src/mip_generation/mod.rs` : `MipGenerationJobs::add` (monde de rendu, SPD compute), `generate_mips_for_phase` à ajouter à la main (exemple `dynamic_mip_generation`) ; `ImageSamplerDescriptor::mipmap_filter` | Mesurer d'abord puis migrer | Petit-moyen / faible (visuel, formats supportés listés par `texture_format_shader_def`, pas les sRGB à vérifier) |
| 2 | Mips des atlas d'impostors | `campaign/vegetation/mips.rs` (298 l.) : pondération par alpha, arrêt à 4 texels pour ne pas mélanger les cellules | idem (1) : filtre box simple, ni pondération alpha ni arrêt par cellule | Garder | -- (le résultat visuel serait dégradé : halos) |
| 3 | Mips/tableaux du sol bataille et détail d'eau | `battle/decor/ground.rs` (1243 l., `mip_chain`, `reinterpret_stacked_2d_as_array`), `campaign/water/detail.rs` (497 l.), `campaign/terrain/ground.rs` | Pour les tableaux : `ImageLoaderSettings::array_layout` (`bevy_image-0.20.0/src/image_loader.rs:159,259`, `ImageArrayLayout`) évite l'empilement manuel. Mips : (1) ; ou cuisson KTX2 (session 56, `processed-assets`) | Ne pas toucher : recouvre la session KTX2 qui cuit les mips hors ligne | -- ; seul `array_layout` est un petit gain, à voir après KTX2 |
| 4 | Instancing d'unités | `battle/units/instancing.rs` (580 l., végétation) + `skinned.rs` (1519 l.) : pipeline `Transparent3d` maison, tampon d'enregistrements 24/48 octets, un draw par (régiment, LOD) | Auto-batching/instancing des entités `Mesh3d` (`bevy_pbr-0.20.0/src/render/gpu_preprocess.rs`), `MeshTag(u32)` (`bevy_mesh-0.20.0/src/components.rs:205`) | Garder | -- |
| 5 | Animation/skinning | `anim.rs` (566 l.) + `skinned.rs` : os cuits en texture (ADR 0014, 0239), clip choisi sur GPU par hash du soldat | `bevy_animation-0.20.0` : `AnimationPlayer`, `AnimationGraph` (`graph.rs`), `SkinnedMesh` : un squelette d'entités par instance | Garder | -- |
| 6 | Fondus / LOD distance | `VisibilityRange` déjà utilisé (`campaign/markers/lod.rs`, `battle/decor/{trees,props,grass,site,features,far_woods,tree_kit}`, `battle/scene/water.rs`) ; fondu par soldat dans le shader (`far_blend`) | `bevy_camera-0.20.0/src/visibility/range.rs` | Déjà natif, sauf unités (voir 4) | -- |
| 7 | Matériaux | `ExtendedMaterial<StandardMaterial, …>` partout (vent, sol, eau, voile, bannière, etc., 14 usages) | `MaterialExtension` | Déjà natif | -- |
| 8 | HUD/UI kit | `ui/kit` (18 fichiers) | `bevy_ui`, `bevy_feathers-0.20.0` | Garder (décision `bevy-misc.md`, aucun fait nouveau) | -- |
| 9 | Caméras | `battle/scene/camera.rs` (554 l.), `campaign/camera/` (rig, zoom, focus) | `bevy_camera_controller` | Garder (même décision `bevy-misc.md`) | -- |
| 10 | Audio | `audio/` (8050 l. total) bâti sur `bevy::audio` (`AudioPlayer`, `PlaybackSettings`, `SpatialListener`, `SpatialScale`, `SpatialAudioSink`) | `bevy_audio-0.20.0` | Déjà natif ; le reste (listes de lecture, banque, voix, ducking) est de la logique de jeu, pas dupliquée | -- |
| 11 | Chargement d'assets | 28 `asset_server.load` ; 4 sites `Image::from_buffer` (`ui/kit/mod.rs`, `campaign/water/{mod,detail}.rs`, `campaign/terrain/ground.rs`) et ~15 fichiers `fs::read` | `AssetServer`, `ImageLoader` | Garder les décodages hors fil (ADR 0051 : calculs hors fil) ; migrer au cas par cas les `fs::read` d'images simples vers `asset_server.load` + `ImageLoaderSettings` seulement lorsqu'un lot touche le fichier | Petit / faible |

## Détails des verdicts « garder »

- Instancing (4) : 65 514 entités au pic (Crécy) ; le goulot est le temps CPU/GPU d'image (`bevy-features.md`). L'auto-batching Bevy garde un coût ECS par entité (transform, visibilité, extraction) que nos tampons d'enregistrements évitent, et `MeshTag` ne porte qu'un `u32` alors que chaque soldat a position, échelle, id, temps d'anim, fondu lointain, temps de salve, surbrillance (48 octets), plus héraldique (atlas en tableau) et livrées en uniform. Variantes par unité, héraldique et vent passent par ces données par instance. Coût de garder : environ 2 100 lignes de pipeline copiées de l'exemple Bevy `custom_shader_instancing`, à réajuster à chaque montée de Bevy. Pistes sans casser ADR 0014 : à la prochaine montée de version, factoriser le code commun de `instancing.rs`/`skinned.rs` (spécialisation, ombres) ; « mesurer d'abord » un banc 65 k entités `Mesh3d + MeshTag` + tampon de stockage pour documenter la comparaison, effort moyen, utile seulement si Bevy retire une API.
- Animation (5) : `AnimationPlayer` anime des entités os ; 65 000 soldats × 22 os = ~1,4 million d'entités, impossible. ADR 0014/0239 (texture d'os cuite) reste le bon choix. Le lot campagne `campaign/life/anim.rs` (512 l.) utilise déjà `MeshTag` + `ExtendedMaterial`.
- Mips impostors (2) : le filtre SPD de Bevy est un box standard ; notre pondération alpha et l'arrêt à 4 texels sont voulus (`mips.rs` en-tête).

## Autres duplications repérées

- Copie du pipeline de rendu PBR dans `instancing.rs`/`skinned.rs` (voir 4) : risque de maintenance, pas de gain de perf à attendre d'un remplacement.
- `mip_chain` / `mip_count` / `mip_levels` : trois implémentations du même calcul (`battle/decor/ground.rs:944,970`, `campaign/water/detail.rs:156`, `campaign/vegetation/mips.rs`). Gain pur de maintenance : un module commun (`fx/model_mips.rs` ou nouveau `fx/mipchain.rs`). Petit, risque nul.

## Classement par gain (maintenance + perf)

1. Factoriser `mip_chain`/`mip_count` (petit, gain maintenance, aucun risque).
2. Mips glTF GPU via `MipGenerationJobs` (petit-moyen) : supprime ~200 lignes et ~20 ms/image de CPU dans le profil dev ; gain perf sur les chargements de campagne. À mesurer (vérifier formats sRGB, RGBA8, compatibilité Metal, `STORAGE_BINDING` à ajouter aux usages d'image).
3. `ImageLoaderSettings::array_layout` pour les tableaux de textures (petit, après KTX2).
4. Factorisation `instancing.rs`/`skinned.rs` (moyen, risque moyen, gain de maintenance lors des montées Bevy).
5. Rien d'autre : UI, caméra, audio, skinning, instancing restent maison.

## Plan en lots (migrations petit/moyen)

- Lot M1 (petit) : module commun de mips (`mip_chain`, `mip_count`) ; tests existants conservés. Aucun ADR touché.
- Lot M2 (petit-moyen) : prototype `MipGenerationJobs` pour les textures glTF (`fx/model_mips.rs`) derrière un réglage data (`ModelMips::Gpu|Cpu`), mesure du temps de chargement campagne (banc `--bench-map`) et comparaison visuelle sur rocs/chariot (une capture, session principale). Garder le chemin CPU si le résultat diffère.
- Lot M3 (petit, après la session KTX2) : `array_layout` à la place de `reinterpret_stacked_2d_as_array`, si les tableaux sont encore chargés en PNG/JPG.
- Lot M4 (moyen, optionnel) : factorisation du code commun `instancing.rs`/`skinned.rs` ; à faire avec la prochaine montée Bevy.

Aucune proposition ne contredit un ADR existant (0014, 0239, 0051, 0357, 0366, 0404 respectés).
