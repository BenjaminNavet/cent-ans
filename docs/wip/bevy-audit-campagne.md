# Audit perf de la campagne Bevy (lecture seule)

Statut (2026-10-22) : audit par lecture de `client/src/campaign/` après fusion de `perf/bevy-par-iter`. Aucune compilation,
aucun lancement, aucun code modifié. Étiquettes : [E] établi par lecture, [S] supposé, [M] à mesurer. Chemins relatifs à
`client/src/campaign/` sauf mention. Mesures de référence : `docs/wip/bm-perf*.md`, `ktx2-assets.md` (pan p99 211 ms / max 1 843 ms,
descente p99 1 689 / max 2 014 ms, graphique 2,8 -> 3,7 Go), `bevy-etude-sol.md`.

## 0. Lecture d'ensemble

- Le gros du travail lourd est déjà hors fil principal : relief (4 décodeurs PNG, `terrain/decode.rs`), sol régional BC1 (`terrain/mod.rs:232`),
  colormap, coast, terroir, végétation (assemblage `vegetation/mod.rs:546`, diffusion `tiles.rs`, `ground_job.rs` sur `AsyncComputeTaskPool`),
  rivières fines (`water/fine`, LRU 40 tuiles), villes lointaines (`towns/far`), countryside (cellules, `SpawnBudget` `countryside/budget.rs`).
  Les meshes du relief et du countryside sont `RENDER_WORLD` seulement (`terrain/mesh.rs:155`, `clutter.rs:270`) : pas de copie CPU.
- Aucun `RenderAssetBytesPerFrame`, `GpuCulling`, ni `par_iter` dans `campaign/` (grep). `PresentMode::AutoVsync` (`main.rs:49`).
- 21 `MaterialPlugin` personnalisés en campagne (`TerrainMaterial`, River, Road, FineRiver/Road, ProvinceFill/Border, TownFar, BuildingAtlas, Kit,
  Banner, Sail, Flag, Anim, Bubble...). Chacun ajoute ses systèmes d'extraction / spécialisation / mise en file.
- Il n'y a PAS de cause unique visible pour les gels de 1,7-2 s dans le code de campagne : voir §2 (hypothèses classées et protocole §6).

## 1. Travail par image au lieu de sur événement

| # | Constat | Gain | Effort | Étiquette |
|---|---|---|---|---|
| 1.1 | `life/models.rs:258 apply_fades` : pour CHAQUE entité à `Fade` (milliers : piétons, charrettes, faune, moulins, fumées, feux) écrit `fade.current` et `transform.scale` par `&mut` à chaque image, même fondu terminé. Tout est donc `Changed<Transform>` en permanence : propagation de hiérarchie (modèles GLB enfants), `GlobalTransform` modifié, ré-extraction au monde de rendu. De plus `life/stream.rs:218 update_targets` écrit `fade.target` par `DerefMut` à chaque image pour tous les `LifeItem`. Correctif : lire d'abord (`if fade.current == fade.target { continue }` avant tout accès mutable), `set_if_neq` pour `target` et `scale`, état « réglé ». | élevé | faible | [E] code, [M] ampleur (nb d'entités : `perf_probe`) |
| 1.2 | `markers/mod.rs:1418 update_stats` (système de la chaîne `Update`, sans cadence) : chaque image parcourt `anchors`, tous les descendants de `MarkersRoot` (`iter_descendants` + `ViewVisibility` de chaque maillage = milliers), `assets.load_state` pour chaque scène et maquette (verrou de l'AssetServer). Sert uniquement au dump. `markers/mod.rs:1339 probe_nearby` (toutes les 10 images, scan de toutes les ancres + chaînes). `lod::batch_stats` (`markers/lod.rs:462`, 0,5 s, construit un `HashSet`). Correctif : calculer dans le dump (`&World`) ou sous drapeau dev. | moyen-élevé | faible | [E] |
| 1.3 | `life/mod.rs:201 update_stats` : toutes les 0,5 s, balayage complet de `LifeItem`, figures, fauna, `AnimBind`... + `BTreeMap` clonés. Pics périodiques de 500 ms. `fx/flipbook.rs:916 count_sprites` : chaque image (aussi en bataille), `clear` + parcours des émetteurs et particules. Même remède (dump seulement). | moyen | faible | [E] ; ce sont probablement les « pointes » vues, pas des artefacts de charge |
| 1.4 | `terrain/parchment.rs:88 sync_wash` : chaque image, reconstruit un `Vec<[u8;4]>` de toutes les provinces (recherche `heraldic.get(owner_of(i))`), compare à l'ancien. Aucune garde `overlay.is_changed()`. | faible-moyen | faible | [E] |
| 1.5 | `water/drape.rs:22 sync_drape::<M>` x 11 types, en `PostUpdate` chaque image : `materials.ids().collect()` (allocation) + comparaison par matériau. Garder par `gpu.is_changed()` ou `AssetEvent::Added/Modified`. | faible | faible | [E] |
| 1.6 | `hud/minimap.rs:742 paint_minimap` : `minimap_colors(...)` (un `Vec` par province) recalculé et comparé chaque image. Dériver d'un compteur de révision de `OverlayState`. Même schéma probable pour `provinces/overlay.rs:427 refresh_overlays` (reste léger entre deux rafraîchissements de 0,25 s grâce aux gardes ; `read_view_wanted` toutes les 0,25 s). | faible | faible | [E] |
| 1.7 | `terrain/look.rs:337 sync_aerial` : écrit `TerrainMaterial` dès que l'atmosphère lissée bouge (fréquent en panoramique). Chaque `get_mut` du matériau terrain = ré-extraction, nouveau buffer d'uniformes ET reconstruction du groupe de liaison à ~35 textures. Regrouper les valeurs par image dans une seule écriture, ou les passer par un petit uniforme séparé. | faible-moyen | moyen | [S] ; [M] compter les `AssetEvent<TerrainMaterial>::Modified` par seconde |
| 1.8 | `armies/skinned.rs:353 update_skinned_armies` : chaque image reconstruit les enregistrements de toutes les armées (octets différents à chaque image car horloge d'animation) y compris hors champ ; aucun test de frustum (`shown` = ni en mer ni masqué par le zoom). Ignorer les armées hors vue / lod lointain, ou ne pousser que celles visibles. `Vec` alloués par groupe et par image. | moyen (selon nb d'armées) | moyen | [E] code, [M] ampleur |
| 1.9 | Végétation : `vegetation/mod.rs:487 assemble_records` est bien événementiel (gardes `moved/density/zone/models`), mais `look::OpenThinning::new` est calculé chaque image avant la garde. Trivial. À l'envoi : `vegetation/billboard.rs:446 prepare_buffers` crée un `Buffer` neuf par lot à chaque `version` (même défaut que `skinned.rs` avant bm-perf3). Réutiliser le tampon persistant (`SkinnedInstances` de `battle/units/skinned.rs`). | moyen (rafales à chaque déplacement du focus) | moyen | [E] code |
| 1.10 | Ombres d'impostors : `vegetation/shadow_pass.rs:224 queue_shadow_batches` re-ajoute / retire tous les lots dans chaque cascade à chaque image. Peu de lots (dizaines) : faible. Hors périmètre à toucher (fichiers d'ombres interdits). | faible | - | [E] |
| 1.11 | `markers/lod.rs:360 fade_maquettes` : déjà optimisé (`fadeband`, quantisation) ; reste l'énigme de `bm-perf4.md` (VisibilityRange constant => GPU 28 ms). Ne pas rouvrir sans mesure GPU. | - | - | - |

Tout le reste des systèmes de `Update` examinés (terrain, vegetation, countryside, towns, water/fine) est gardé par état, révision ou budget.

## 2. Travail lourd sur le fil principal (gels de 1,7-2 s)

Établi : aucun décodage PNG/JPG, aucune lecture de fichier ni construction d'atlas n'a lieu dans un système de `Update` de campagne après l'entrée
(les `std::fs::read*` de `water/`, `markers/`, `provinces/`, `towns/layer.rs:~255` sont dans des `OnEnter`/`Startup`, donc au chargement, pas pendant
la descente). Les gels ne viennent donc probablement pas d'E/S de notre code. Pistes classées :

| # | Hypothèse | Étiquette |
|---|---|---|
| 2.1 | Envoi GPU en une image de grosses textures : `RenderAssetBytesPerFrame` non réglé, donc `terroir::finish`, `coast::bind_coast`, fond/parcelles (`terrain/mod.rs:415-422`), wash/hatch/fill/field/clear (`images.get_mut` re-crée une texture GPU entière à chaque modification, 10,8 Mo pour le fill 1770x1526), et surtout `relief_shade` BC5 (224 Mo) s'uploadent sans limite. Bevy 0.20 : `RenderAssetBytesPerFrame::new(n)` (`bevy_render-0.20.0/src/render_asset.rs:509`) étale les envois (mise à disposition plus tardive, mais pas de gel). | [S] fort ; [M] |
| 2.2 | Rafales d'entités : `calculate_bounds` + `check_entities_needing_specialization` (150 ms mesurés au début de la descente, `bm-perf.md`) ; `SpawnBudget` n'existe que dans countryside. Étendre à `life` (`folk::spawn_walkers`, `spawn_herds`, `spawn_sites`), `markers::build_markers`, `armies::sync_markers`, `towns` (les fonctions `spawn_*` n'ont pas de budget par image). Chaque `WorldAssetRoot` de GLB chargé d'un coup déclenche aussi la copie de scène sur le fil principal. | [S] ; [M] |
| 2.3 | Compilation de pipelines : un changement de clé de spécialisation (nombre de cascades via `shadows::size_shadows`, MSAA `settings/quality.rs`, brouillard) recompile `TerrainMaterial` (shader WESL de 2 401 lignes, ~35 liaisons) : sur Metal, naga -> MSL -> `newLibrary` prend de l'ordre de la seconde. Bevy compile en tâche asynchrone sauf `synchronous_pipeline_compilation` (non réglé) donc ce serait du clignotement plutôt qu'un gel, sauf si le pilote Metal bloque. | [S] faible-moyen |
| 2.4 | Pression mémoire machine : 7,4 Go d'empreinte, graphique en hausse, charge machine 11-31 : les gels de ~2 s sont typiques d'un défaut de page / compression mémoire macOS. Corrélable avec §3. | [S] ; [M] `vm_stat`/`footprint` pendant le banc |
| 2.5 | Tâches sur le fil principal à effet unique, de l'ordre de 50-200 ms chacune : `weather/ground.rs:226 bake_clear` (à chaque changement de province sélectionnée : 1770x1526 = 2,7 M texels x 5 lectures de `index_at`, sur le fil principal, puis nouvelle texture) ; `provinces/overlay.rs:343 paint_fill` x 3 (`fill`, `hatch`, `wash`) à chaque changement de couleurs (fin de tour, changement de mode) ; `hud/minimap.rs:742` `bake_relief` à la première image prête ; `terrain/lod.rs:228 update_lod` (budget 6 ms, mais `built > 0` autorise toujours un nœud : un nœud coûte combien ? [M]). Aucune ne fait 2 s, mais elles s'additionnent lors d'un clic + tour. Mettre `bake_clear` et `paint_fill` sur `AsyncComputeTaskPool` (ou sur la grille de masque en GPU). | [E] pour l'existence, [M] durée |
| 2.6 | Interblocage / attente de fil de rendu signalés dans `bm-perf3.md` (sortie seulement). Fils ad hoc : ~14 `std::thread::spawn` + 4 décodeurs + pool de diffusion végétale en plus des pools Bevy => sur machine chargée, concurrence avec le fil de rendu. Passer ces tâches sur `AsyncComputeTaskPool`/`IoTaskPool` (priorité contrôlée). | [S] faible |

## 3. Croissance continue de la mémoire graphique (2,8 -> 3,7 Go)

Aucune fuite évidente dans la campagne : les nœuds du relief rendent leurs `Mesh` en remplaçant `Mesh3d` (`terrain/lod.rs:410`), les nœuds retirés sont
détruits (`:428`), les caches sont bornés (256 pages ~134 Mo de `u16` CPU `lod.rs:25`, 40 tuiles de rivière, 96 cellules d'herbe `clutter.rs:58`,
14 000 entités countryside `layer.rs:34`). Hypothèses, par ordre de probabilité :
1. [S] Montée de « mise en route » : GLB/textures chargés à la demande (modèles d'arbres `vegetation/render.rs`, figures de `life`, protos countryside
   `proto.rs`, héraldique, atlas de villes) jamais déchargés. Se stabilise après un parcours complet : un banc de 20 s est trop court. [M] banc de 10 min
   en boucle pan/descente : si la courbe plafonne, ce n'est pas une fuite.
2. [S] Fragmentation du `MeshAllocator` de Bevy : les tranches (slabs) ne se rétrécissent jamais ; les `meshes.insert(handle, mesh)` de taille variable
   (`clutter.rs:681` rebâtit la cellule quand le relief change) et les créations/libérations de maillages de tailles diverses (rivières fines, ponts,
   `towns/far`) peuvent faire grandir les slabs jusqu'à 512 Mo. Le relief (nœuds de taille identique) n'en souffre pas. [M] ajouter au dump le nombre
   et la taille des slabs (`MeshAllocator::slab_count`/stats si exposé, sinon `Assets<Mesh>::len`).
3. [S] Re-création des textures GPU à chaque modification d'image (`images.get_mut` sur fill, hatch, wash, field, clear, minimap) : le `GpuImage`
   précédent est détruit mais wgpu libère en différé ; le pic peut dépasser plusieurs fois 10 Mo et gonfler l'empreinte mesurée sans fuite réelle.
4. [S] Buffers d'instances recréés à chaque version (§1.9) : même remarque, sur la végétation.
5. [S] `RenderAssetUsages::default()` (MAIN|RENDER) sur les images de `provinces/overlay.rs:177,198,224,307`, `borders.rs:266`, `markers/banner.rs:162`,
   `council/figures.rs:121` : copie CPU conservée (RAM, comptée comme mémoire unifiée). Passer à `RENDER_WORLD` quand le CPU n'a plus besoin de relire.
6. [M] `bevy-texcensus` (sections `textures`/`texture_loads` du dump) n'a jamais été compilé (disque plein) : c'est l'outil prévu pour trancher.

## 4. Draws, entités, matériaux, cascades, passes

- [E] Relief : 350-900 nœuds (`relief_items`, `settings/quality.rs:80`: 700), UN matériau, UN mesh par nœud (pas d'instanciation). **Tous portent
  `NoFrustumCulling`** (`terrain/lod.rs:368`) ; or `bevy_light-0.20.0/src/lib.rs:~455` n'exclut un objet d'une cascade que `!has_no_frustum_culling && !intersects`.
  Donc chaque nœud est dessiné dans les 4 cascades du soleil (`map_shadow_splits`, `terrain/shadows.rs:42`) : jusqu'à ~3 600 draws d'ombre (prépasse sommet
  `terrain_shadow.wesl`, 65x65 sommets + jupes) au lieu d'environ 1 000. Piste : retirer `NoFrustumCulling` des nœuds et corriger leur `Aabb` (le commentaire
  de `lod.rs:362` attribue les trous à une boîte fausse ; la boîte `-10..500` en y est peut-être trop basse une fois l'exagération verticale appliquée) ;
  ou une sélection par cascade faite par le CDLOD. Gain probable = fraction notable des ~5 ms d'ombres restantes. [S] pour le gain, [M] à vérifier
  avec `CENT_ANS_PERF_OFF=terrain` combiné à `shadows` (voir §6) et compte de phase-items Shadow.
- [E] 21 types de matériau : surcoût fixe mesuré 0,5-0,8 ms/type pour `check_entities_needing_specialization` (`bm-perf.md`) soit ~5-10 ms CPU cumulés si
  ce chiffre est exact pour chaque type peuplé. Fusionner les matériaux rares (rivières/routes fines/grossières, Province Fill/Border) ou, avec Bevy 0.20,
  vérifier que les types sans entité ne coûtent rien. [M] profil `profile` par système.
- [E] Cascades : 4 (`CascadeShadowConfigBuilder` dans `terrain/shadows.rs:103`), tailles via `cascade_map_size` (4096 en haute, p50 12,3 ms contre 10,0 à 1024).
  Garder `map_shadow_splits`/`shadow_map_size` distincts pour la campagne (déjà : `map_shadow_cascades`) ; un réglage taille carte campagne séparé de la bataille
  n'existe pas. Coût d'ombre résiduel ~5 ms sur 12,6 ms (`bm-perf4.md`).
- [E] Caméras/passes : une `CampaignCamera` 3D + `UiCamera` 2D + passe de post `terrain/post.rs` (ajustement `CampaignAdjust` après MSAA) ; MSAA 2x en campagne
  (`settings/quality.rs:440`) ; pas de prépasse (`enable_prepass() = false`, `terrain/material.rs:188`).
- [S] Entités : ~49 000 (`fix-perf.md`) dont 8 059 murets / 4 194 cabanes de vigne au tour 4 ; compter par archétype via `perf_probe` en campagne à d=30 et d=300.
- [M] Draws réels : aucun compteur Bevy ; ajouter (§6).

## 5. Ce que Bevy 0.20 permet de mieux

| Levier | Où | Gain | Remarque |
|---|---|---|---|
| `RenderAssetBytesPerFrame::new(8-32 Mo)` | `app.rs`/plugin render | moyen-élevé sur les gels, nul sinon | une ligne, réversible ; mesurer latence de première apparition des textures |
| Retirer `NoFrustumCulling` du relief + `Aabb` exacte | `terrain/lod.rs:368` | moyen (ombres) | risque de trous (déjà vécu) |
| `GpuCulling` + `NoCpuCulling` sur la caméra | caméra campagne | faible-moyen (CPU de `check_visibility` sur ~49 k entités) | incompatible avec les rendus non-mesh personnalisés (billboards, skinned) ? [M] ; les VisibilityRange des maquettes doivent rester |
| `Changed<T>` / `set_if_neq` | `life/models.rs`, `life/stream.rs`, `armies/markers.rs` | élevé (§1.1) | |
| `par_iter_mut` | `life::move_walkers/move_carts/move_animals`, `fade_*`, `update_skinned_armies` (calcul des enregistrements) | moyen | uniquement après avoir supprimé le travail inutile ; machine chargée => gain réduit |
| `VisibilityRange` | déjà utilisé (maquettes, 1 176) | - | énigme GPU de `bm-perf4.md` |
| Tampon persistant (`write_buffer_with`) | `vegetation/billboard.rs:460` | moyen | reprendre `SkinnedInstances` |
| Matériau de relief commun en `Material` standard étendu / array texture (`ImageLoaderSettings.array_layout`) | sol | voir `bevy-etude-sol.md` | hors périmètre |

## 6. Plan de mesure (aucune exécution ici)

Interrupteurs existants (`CENT_ANS_PERF_OFF`, `campaign/perf_switch.rs`) : `shadows,post,veg,vegshadow,terrain,water,provinces,towns,life,armies,markers,psea`,
plus `fadeband`, `spawnbudget`, `vslod1`. Banc : `BENCH_FEATURES=profile tools/bevy_bench/bench.sh <nom>` (release, hors écran, muet), sur machine calme
(`uptime` consigné), 3 essais.
1. **Référence** après la dernière fusion : pan + descente d=30, noter p50/p95/p99/max, nb d'images > 100 ms, tout `profile` activé.
2. **Journal de pics** (instrumentation à ajouter, hors des fichiers interdits) : sur toute image > 100 ms, journaliser pour la frame (a) le système principal le plus long
   (spans `ecs_trace`), (b) `lod.pending_builds`, nb d'entités créées, nb de `Assets<Image>`/`Assets<Mesh>` modifiés (compteurs `AssetEvent`), (c) octets uploadés par
   `RenderAssetBytesPerFrameLimiter`, (d) temps `RenderApp` par schedule. Donne enfin la cause des gels de 1,7-2 s.
3. **Bissection du CPU** : `profile` seul (sans changement) puis `PERF_OFF=life`, `markers`, `armies`, `towns`, `water` un par un : on attend `life` et `markers` en tête
   (§1.1-1.3). Mesurer aussi le temps en `PostUpdate` de propagation/visibilité avant/après le correctif 1.1.
4. **Bissection ombres du relief** : `PERF_OFF=terrain` (déjà dispo) vs base à qualité haute, puis un essai `CENT_ANS_SHADOW_SIZE=2048` et un essai à 2 cascades
   (`map_shadow_splits=2`) pour isoler la part de la prépasse sommet des nœuds ; compter les phase items `Shadow` (ajouter dans le dump : taille de la phase par vue lumineuse).
5. **Mémoire** : banc long (10 min) en boucle ; relevé toutes les 10 s de l'empreinte (`footprint`/`vmmap -summary`), `Assets<Image>/Mesh/Material` (`perf_probe`), `texture_census`
   (compiler d'abord `bevy-texcensus`), et ajouter au dump : nombre/volume de slabs du `MeshAllocator`, modifs d'images par minute. Un plateau = pas de fuite.
6. **Gels vs upload** : A/B `RenderAssetBytesPerFrame::new(16 Mo)` ; si le max d'image chute sous 300 ms, c'était l'upload.
7. **Draws** : ajouter au dump le nombre d'items par phase (`Opaque3d`, `AlphaMask3d`, `Transparent3d`, `Shadow` par cascade, billboards/skinned) via
   `RenderDiagnosticsPlugin` (feature `bevy/render_dev_tools` déjà dans `dev`).

## 7. Lots de correction proposés (ordre de rentabilité)

- **L1 (élevé, faible)** `life` : garde de `apply_fades`/`update_targets` (`set_if_neq`, saut des fondus terminés). + rechercher le même schéma dans `birds`, `effects::animate_*`
  (écritures de `Transform` sans test de changement ni de distance). Test unitaire : un fondu réglé ne marque plus `Changed<Transform>`.
- **L2 (moyen-élevé, faible)** Sortir les statistiques de débogage de la boucle : `markers::update_stats`, `probe_nearby`, `batch_stats`, `life::update_stats`,
  `flipbook::count_sprites` -> calculées dans le dump (`&World`) ou derrière un drapeau dev / `run_if`.
- **L3 (moyen, faible)** `RenderAssetBytesPerFrame` + `Image` `RENDER_WORLD` partout où le CPU ne relit pas ; gardes de §1.4-1.6 (`sync_wash`, `sync_drape`, `paint_minimap`).
- **L4 (moyen, moyen)** Tâches asynchrones pour `bake_clear`, `paint_fill` (fill/hatch/wash), `bake_relief` de la minimap ; budget par image des `spawn_*` de `life`, `markers`, `armies`.
- **L5 (moyen, moyen)** Relief : retirer `NoFrustumCulling` + boîtes exactes, ou culling par cascade ; mesurer le temps de prépasse d'ombre avant/après ; protéger par test de trous.
- **L6 (moyen, moyen)** `update_skinned_armies` : ne mettre à jour que les armées visibles ; tampon persistant pour `vegetation/billboard.rs`.
- **L7 (à décider après §6)** Fusion de matériaux rares ; `par_iter` ; `GpuCulling` ; étude `MeshAllocator`.

## 8. Limites de l'audit
Lecture seule : aucun chiffre nouveau. Non lus en détail : `hud/` (hors minimap), `council/`, `progress/`, `camera/`, le détail de `weather/` hors `ground.rs`, `provinces/labels`/`shields`
(le coût des étiquettes avait été corrigé par `bm-perf.md`), `countryside/rocks|understory|decor`. Les pointes citées (`update_skinned_armies`, `refresh_overlays`, `life::update_stats`,
`count_sprites`) sont expliquées par §1.8, §1.6, §1.3 : elles sont réelles (travail par image) mais amplifiées par la charge machine.
