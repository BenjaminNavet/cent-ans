# Étude : refaire le sol (bataille + campagne) plus près de Bevy et plus performant

Statut (2026-10-22) : étude en lecture seule, aucun code, aucune compilation. Auteur : lot « sol ».
Réserve de méthode : `git merge --ff-only perf/bevy-par-iter` a échoué (la branche du worktree avait 3 commits d'avance
sur `perf/bevy-par-iter`, déjà dans main) ; le `reset` a été refusé par le garde-fou. Tout est donc lu sur la base main
`292adf96f`, sans les commits perf (aucun ne devrait toucher `ground.rs` / `terrain.wesl` : non vérifié).
« Mesuré » = chiffre trouvé dans un document du dépôt ; « supposé » = déduction de lecture, à confirmer au banc.

## 1. Ce que fait le pipeline actuel

### Bataille (`client/src/battle/decor/ground.rs`, `ground.wesl`, `scene/terrain.rs`)
- **Maillage** : un seul mesh `BattleTerrain` (`scene/terrain.rs:199`), un sommet tous les `field.resolution / 2` m
  (`SUBDIVISION = 2`), poids de splat par sommet (COLOR = herbe/forêt/terre/boue, UV_1 = pierre/labour). Pas de LOD.
  Un draw (plus les cascades d'ombre). Ordre de grandeur supposé : 5 m de pas, ~40 k sommets sur 1200 x 800 m.
- **Matériau** : `ExtendedMaterial<StandardMaterial, GroundExtension>`, forward, seul `fragment_shader` surchargé.
  Liaisons : albedo `texture_2d_array` (13 couches 1024², bande JPG 1024 x 13312), normales (256², bande 256 x 3328),
  grain micro (grille 2 x 4 de 1024², albedo + normales), carte des champs 4 m, carte de relief R2, carte de piétinement.
- **Construction CPU** (`PendingStrips` / `convert_strips`, `PendingGrids` / `convert_grids`, système `Update`, fil
  principal) : décodage JPG par le loader Bevy (RGBA8), `grid_to_strip` (copie de réarrangement du grain micro),
  `strip_to_array` : chaîne de mips par boîte 2 x 2 octet par octet (`mip_chain`, boucle scalaire à 3 niveaux
  d'imbrication), copie dans un second `Vec`, `reinterpret_stacked_2d_as_array`, échantillonneur Repeat + anisotropie.
  Temps non mesuré (le recensement `texture_loads` de `bevy-texcensus` n'a jamais été compilé : disque plein).
  Supposé : quelques centaines de ms sur le fil principal, donc un accroc à l'ouverture de la bataille.
- **Mémoire** (calcul, mips compris, RGBA8) : albedo 13 x 5,33 Mio = **72,7 Mio** ; normales 256² : 4,3 Mio ; grain micro
  (8 couches 1024², albedo + normales) : ~85 Mio ; cartes : < 3 Mio. Soit ~160 Mio de VRAM. Les images gardent par défaut
  `RenderAssetUsages::MAIN | RENDER` (le code ne le change pas) : la même quantité reste en RAM (supposé ; le recensement
  prévu devait le confirmer). Machine à mémoire unifiée : les deux comptent.
- **Échantillons par fragment** (lu dans `ground.wesl:339-470`) : boucle sur 13 couches ; pour chaque couche de poids
  >= 0,004 : 2 prises albedo (deux échelles/orientations) + 2 prises normale, `textureSampleGrad` anisotrope = **4 prises
  par couche**. Plaine ouverte : 2 à 4 couches actives (herbe, prairie, fleurs, piétiné) ; mélanges champ/neige/boue :
  jusqu'à 6-8 (supposé). Ajouts : détail proche (2 prises, < ~117 m), grain micro (3 prises, < 32 m), carte des champs
  (1 + 4 dans `decor_clear_soft`), relief 1, piétinement 1. Soit ~12-25 prises typiques, 40+ au pire, plus ~15 évaluations
  de bruit de valeur (`macro_tex`, `bt_value_noise`, 3 x `detail_height` pour les bosses) : le fragment est surtout **ALU**
  (supposé, pas de profil GPU).
- **Gaspillage repéré** : la boucle ne s'arrête jamais selon la distance. `far_k` ne fait que *mélanger* vers la moyenne
  (max 0,8), et au-delà de `detail.y` (140 m) `strength` vaut 0 : les 2 prises de normale par couche sont multipliées par
  zéro sur toute la moitié lointaine de l'écran (`ground.wesl:219-221`, `strength` ~l.505). Aucun des deux n'est une
  optimisation que le compilateur puisse déduire.
- **Prépasses / différé** : SSAO impose profondeur + normales (`scene/lighting.rs:28`) avec le prépasse *par défaut* du
  StandardMaterial (sommet seul, normales géométriques). Aucune `OpaqueRendererMethod` n'est fixée nulle part.

### Campagne (`client/src/campaign/terrain/`)
- **Maillage** : quadtree CDLOD du crate `relief-lod` (`lod.rs`), un nœud = un patch de 64 x 64 quads + jupe (32 pour un
  quadrant, `mesh.rs`), **maillé sur le CPU** depuis les pages résidentes (`Surface::node_height_m`, budget
  `BUILD_BUDGET_MS = 6 ms/image`), une entité et un `Mesh` par nœud ; budget de nœuds 350-900 selon la qualité
  (`relief_items`, doc `vq-cperf.md`) ; 1,5-2 M de triangles en ultra avant réglage. Pages : tuiles 16 bits 512 px décodées
  par 4 fils (`MAX_JOBS`), 256 pages résidentes au plus (`MAX_PAGES`). Géomorphage via COLOR (delta d'altitude parent),
  exagération verticale dans le vertex shader (`terrain.wesl`, texture de plancher Rgba32Float). ADR 0092, 0353, 0359.
- **Draws** : ~nœuds (350-900) + cascades d'ombre (prépasse sommet `terrain_shadow.wesl`) ; chaque nœud un mesh distinct
  donc pas d'instanciation (supposé : le batching Bevy ne fusionne que mesh + matériau identiques ; `MULTI_DRAW_INDIRECT`
  absent sur Metal, non vérifié).
- **Matériau** : `Material` entièrement maison (`TerrainMaterial`, `terrain.wesl` 2401 lignes, éclairage, brouillard,
  ombres solaires, nuages, saisons dans le shader), `enable_prepass() = false`, ombres via prépasse sommet. ~35 liaisons
  de texture (carte de couleur BC1 ADR 0118, relief_shade BC5 224 Mio, splat, landuse, terroir, coast, wx, occlusion...).
- **Construction CPU du sol régional** (`ground.rs`, fil de chargement, `std::thread::scope`) : planche JPG 5120 x 9216 =
  45 couches 1024² ; albedo **encodé en BC1 sur le CPU** (`bc1.rs`) avec chaîne de mips par couche, normales Rg8 <= 1024
  (512 ici) ; variante locale 2048 en haute qualité. Mémoire : albedo 45 x 0,67 Mio = 30 Mio, normales 30 Mio, plus
  parcelles (`tx_campaign_parcels`, 22 + 11 Mo JPG) et grain micro. Temps non mesuré.
- **Échantillons** (`terrain.wesl:1124-1180, 1776-1860`) : `sample_layer` = 4 prises (albedo + normale à 2 échelles) par
  rôle actif (poids > 0,01), **doublées** quand le biome voisin diffère (`sample_role`) ; 2-3 rôles typiques donc 8-12 prises,
  16-24 en bordure de biome ; + grain micro, splat, landuse, terroir, ab_dist x3, occlusion, parchemin (wash x5), côte (x5+),
  météo, brouillard de guerre, parcelles/champs peints (`hb_sample`, 4 prises par couche). 30 appels statiques de
  `textureSample*` dans `fragment` seul. Ordre de grandeur supposé : 30-60 prises à zoom proche.
- **Gaspillage repéré** : quand `det = tiling.z * detail_fade` vaut 0 (empreinte > `fades.y` = 1,3 px de carte par pixel),
  `sample_layer` ne contribue plus rien (`mix(1, ratio, 0)` et `dn * detail_fade = 0`) mais les 4-8 prises sont faites
  quand même ; idem `micro_albedo` quand `micro_k = 0` (au-delà de l'empreinte 0,02, ligne 1850). C'est la grande majorité
  des pixels aux zooms de jeu.
- **Mesure existante** (`docs/wip/bm-perf4.md`) : bissection GPU `CENT_ANS_PERF_OFF` : sans ombres 7,7 ms contre 28 ms ; sans
  `terrain` (masquage de `TerrainMaterial`), post, eau, provinces : **inchangé 27-28 ms**. Le sol de campagne n'est donc pas
  le goulot au panoramique d=30 (limité par le remplissage des ombres). Aucune bissection équivalente pour la bataille.
  Gels de campagne (`ktx2-assets.md`) : p99 1,7-2 s sur quelques images, cause non établie (chargements sur le fil principal).

## 2. Options plus proches de Bevy 0.20 (lu : `bevy_pbr-0.20.0`, `bevy_image-0.20.0`, `bevy_core_pipeline-0.20.0`,
`bevy-0.20.0/examples`, `_release-content/release-notes`)

Ce que Bevy 0.20 apporte réellement :
- `ImageLoaderSettings.array_layout: Option<ImageArrayLayout>` (`image_loader.rs:105,159`, variantes `RowCount`,
  `RowHeight`, `GridCount`, `GridSize`) : empile une planche en tableau *au chargement*. Remplace `grid_to_strip` +
  `reinterpret_stacked_2d_as_array` ; ne crée pas de mips. Exemple `examples/shader/array_texture.rs` (+ `MeshTag`).
- Loader KTX2 avec `layer_count`, zstd, transcodage Basis -> BC7/BC5 (`ktx2.rs`) ; `CompressedImageSaver` (feature
  `compressed_image_saver`, bibliothèque `ctt`, BC7/BC5/BC4, **mips générés**, notes `compressed_image_saver.md`).
  Prise en charge des tableaux par `ctt.rs` (`layer_data`) : à confirmer par un essai.
- `MipGenerationPlugin` / `MipGenerationJobs` (`bevy_core_pipeline/mip_generation`, exemple `2d/dynamic_mip_generation.rs`) :
  mips **sur GPU** (FidelityFX SPD, compute), support `ARRAY_TEXTURE`, formats Rgba8Unorm, Rg8Unorm... mais pas sRGB (il faut
  une vue Unorm + flag `STORAGE_BINDING`) ; système de rendu à ajouter à la main. Moyenne en espace gamma, comme `mip_chain`.
- Bindless (`#[bindless]`, `extended_material_bindless.rs`) : sert à *fusionner les ressources de nombreux matériaux* dans
  un seul bind group pour batcher les draws ; ne réduit pas les prises par fragment. `ExtendedMaterial` n'est bindless que si
  base **et** extension le sont (`extended_material.rs:181`) ; notre extension ne l'est pas.
- `StandardMaterial` : parallax / POM (`max_parallax_layer_count`, défaut 16) est une prise de plus par couche de marche, donc
  plus cher ; aucun tableau de textures ni splat.
- **Rien de natif pour le terrain** : grep `clipmap|virtual texture|heightmap|tessellat` dans bevy_pbr/render/mesh/core_pipeline/
  solari : aucun résultat (seul `parallax_mapping.wesl`). Pas de terrain, clipmap, virtual texturing, ni displacement. Les
  « mesh shaders » 0.20 (`mesh_shaders.md`) sont une API bas niveau (`MeshPipeline`), non intégrée au rendu de matériaux ;
  support Metal via wgpu : non vérifié.
- Solari (`bevy_solari`, `solari_metal.md`) : voir `bevy-solari.md` ; `SolariLightingPlugin` impose le rendu différé par défaut
  et seuls `StandardMaterial` + `RaytracingMesh3d` entrent dans la TLAS. Pas de débruiteur sur Apple.

### Tableau des options

| # | Option | Gain attendu | Solari / différé | Effort | Risque visuel |
|---|--------|--------------|------------------|--------|---------------|
| A | **Garde par distance dans les shaders** : bataille, sauter les 2 prises de normale par couche quand `strength` = 0 (> 140 m), sauter la boucle de couches au-delà de `texture_fade_m * 1.6` (~830 m, résultat = moyennes), ne faire qu'une prise si `blend` est saturé (0 ou 1) ; campagne, sauter `sample_layer` si `det <= 0` et `micro_albedo` si `micro_k <= 0` | Prises de couche -50 % (normales) au-delà de 140 m en bataille ; campagne 8-24 prises -> 0 sur les pixels hors détail. Mémoire et chargement : 0. | Neutre (même sortie) | Petit | Nul à négligeable (résultats mathématiquement identiques, sauf `blend` saturé : < 1/255) |
| B | `RenderAssetUsages::RENDER_WORLD` sur les tableaux du sol (bataille et campagne) | jusqu'à -160 Mio de RAM par bataille (albedo 73 + normales 4 + grain micro ~85 ; supposé : la copie CPU subsiste) | Neutre | Petit (vérifier que la conversion `Update` se fait avant le déchargement CPU) | Nul |
| C | `array_layout` dans `ImageLoaderSettings` au lieu de `grid_to_strip` + reinterpret | -1 copie de 54 Mio + code en moins ; mips toujours CPU | Neutre | Petit | Nul (vérifier l'ordre des cellules Godot `RowCount` vs grille) |
| D | **Cuisson hors ligne en KTX2 tableau** (albedo BC7 sRGB, normales BC5, mips inclus ; idem grain micro ; campagne BC7/BC5 à la place de l'encodeur BC1 maison et du Rg8) chargé directement | Construction CPU ~0 (plus de JPG géant, `mip_chain`, copie, ni encodage BC1 sur fil) ; VRAM bataille 160 -> ~40 Mio (BC7 = 1 o/texel : albedo 17 Mio, micro ~11 Mio) ; campagne normales 30 -> 15 Mio. Prises : inchangé. | Neutre ; le différé n'impose aucun format | Moyen | Faible (BC7 >= qualité du BC1 maison ; vérifier la teinte sRGB et le mip 9 de `far_flatten`) |
| E | Mips sur GPU (`MipGenerationPlugin`) à la place de `mip_chain` | Supprime le coût CPU des mips ; mémoire inchangée ; code de rendu à écrire | Neutre | Moyen | Faible (sRGB : vue Unorm) ; dominé par D |
| F | Plafond de N = 3 couches par fragment (tri partiel des poids, renormalisation) derrière un réglage `data/` | Pire cas bataille 6-8 couches (24-32 prises) -> 12 ; cas typique inchangé | Neutre | Petit-moyen | Moyen (apparition de bords là où la 4e couche pèse 5-15 %) |
| G | Poids de répartition cuits dans une carte de contrôle (indices + poids RGBA8, « splat 4 couches + indirection ») | Prises bornées à 4 couches ; ALU de piles de poids en moins | Neutre | Gros | **Élevé** : les poids actuels sont bruités par pixel (`fine`, `macro_n`, `patch_n`...) ; une carte à 1-2 m perdrait la rupture de motif identitaire |
| H | Sol différé-compatible : scinder `fragment` en `ground_pbr_input()` + écriture G-buffer (`deferred_fragment_shader`, `deferred_output`) ; `opaque_render_method` explicite | Prérequis Solari / SSAO correcte (normales par couche) ; coût GPU 0 ou léger | **Indispensable** : sans cela, avec `DefaultOpaqueRendererMethod::deferred()` le sol retombe sur le shader différé du StandardMaterial (`extended_material.rs:294`, repli sur `B::`) : sol blanc/plat | Moyen | Faible si la sortie forward reste la référence |
| I | Cuisson à l'exécution du « champ lointain » : rendre une fois le sol dans une texture (caméra orthographique, 0,5-1 m/texel) et l'échantillonner (1 prise) au-delà de ~140 m ; détail procédural seulement proche | Ordre de grandeur : fragment lointain = 1 prise + quelques ALU ; mémoire +16-32 Mio | Cuisson en forward OK ; la texture n'entre pas dans la TLAS | Gros | Moyen-élevé (neige/piétinement dynamiques à re-cuire, raccord proche/loin) |
| J | Campagne : patch partagé déplacé dans le vertex shader depuis un tableau de pages (comme Godot) avec instanciation par `Transform`/`MeshTag`, au lieu de maillage CPU par nœud | 350-900 draws -> 1-2 ; plus de budget CPU 6 ms/image de maillage ; hauteurs CPU (`GroundHeight`) inchangées | Hors sujet (matériau forward maison, sans Solari) | Gros | Moyen (hauteurs, normales, géomorphage, jupes à reproduire à l'identique) |
| K | `StandardMaterial` + parallax, bindless, terrain/clipmap/VT natif | néant | — | — | **Écartés** : rien de natif, POM plus coûteux, bindless sans effet sur le nombre de prises |

Remarque campagne : `TerrainMaterial` est un `Material` maison avec éclairage propre ; il ne peut pas devenir différé sans
réécrire son éclairage sur `PbrInput`. Si `SolariLightingPlugin` est ajouté, il faut fixer `opaque_render_method() = Forward`
sur tous les matériaux maison (sol bataille, sol campagne, plantes, soldats instanciés) : aucun n'est fixé aujourd'hui
(grep `OpaqueRendererMethod` vide dans `client/src`). Point ouvert, à confirmer avec le lot Solari.

## 3. Recommandation

1. **Mesurer d'abord** (lot S0) : on ne sait pas si le sol de bataille est limité par l'ALU, les prises ou l'ombre. Sans cela
   A-K ne sont que des paris.
2. Enchaîner les gains **exacts et peu risqués** : A, B, C. Puis D (le vrai chantier mémoire/chargement), en coordination avec
   la session KTX2 (ne pas toucher `asset_processing.rs` ni la feature `processed-assets` : D se fait avec un script
   `tools/` + paramètres de chargement, pas via `AssetMode::Processed`).
3. F seulement si S0 montre un fragment lié aux prises avec des pics dans les mélanges ; G, I, J seulement si A + D laissent
   un coût mesuré. H dès que le lot Solari sort du prototype (le sol est visible sur plus de la moitié de l'écran).

### Plan en lots
- **S0 (mesure, petit)** : interrupteur de bissection côté bataille (équivalent `CENT_ANS_PERF_OFF=terrain` de
  `campaign/perf_switch.rs` : sol remplacé par un `StandardMaterial` uni) ; `bench.sh <label> --bench-battle --offscreen`
  (Crécy, calme puis mêlée, `out/bench/battle_<label>.json`) en 3 essais alternés avec `tools/bevy_bench/ab.sh` ; ms GPU par passe
  via `bevy_render` diagnostics (supposé disponible, `bevy_pbr/src/diagnostic.rs`) ; temps de construction : sections de dump
  `textures` / `texture_loads` de `bevy-texcensus` (déjà écrites, à compiler) ou `Instant` autour de `strip_to_array`.
  Prototype minimal : jalon « sol uni vs sol procédural » = coût GPU du sol en ms.
- **S1 (A, petit)** : gardes de distance bataille + campagne. Vérification : test `ground_shader_validates` (naga) ; A/B
  `bench.sh` (ms) ; différence de pixels via `tools/compare_views.sh` (vues `tools/refshots/views.txt`, rendu hors écran) avec
  seuil numérique (écart max <= 1/255 attendu), sans lire d'image.
- **S2 (B + C, petit)** : usage de rendu seul + `array_layout`. Vérifier par le dump `textures` (octets CPU retenus) et RSS.
- **S3 (D, moyen)** : outil de cuisson (`ctt` ou `ktx create --layers`, ADR 0724 : cache généré non committé, repli sur
  JPG/PNG si absent) ; chargeur qui préfère le KTX2 ; supprimer ensuite `mip_chain` / `strip_to_array` / `bc1.rs` du chemin
  chaud (garder en repli). Mesure : temps d'ouverture bataille et de chargement campagne (`texture_loads`), RSS, mémoire
  graphique du banc `mem-battle` (1,2 -> 2,4 Go, relevé 3,6 Go de pointe : référence du jour, `ktx2-assets.md`).
- **S4 (H, moyen)** : sol différé-compatible, derrière le lot Solari ; test `naga` des deux variantes.
- **S5 (F / I / J)** : seulement sur décision après S1-S3, chacun derrière un réglage `data/` désactivé par défaut.

## 4. Points ouverts / suppositions à lever
- Coût GPU réel du sol de bataille (aucune mesure) ; part ALU contre prises.
- Temps de construction CPU des tableaux (instrumentation écrite mais jamais exécutée).
- Copie CPU conservée après envoi (supposé) ; effet de `RENDER_WORLD` sur la conversion en `Update`.
- `ctt` gère-t-il une entrée en tableau de couches ? Ordre des cellules de `ImageArrayLayout::GridCount` contre `grid_to_strip`.
- Sol de campagne sous Solari/différé par défaut : matériaux maison à passer en forward (lot Solari).
- Cause des gels de campagne de 1,7-2 s : pas établie (ni le sol ni le maillage ne sont prouvés coupables).
