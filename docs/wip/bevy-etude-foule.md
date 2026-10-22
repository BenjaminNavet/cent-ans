# Étude : rendu et animation de masse des soldats, plus près de Bevy ?

Lecture seule (aucune compilation, aucun lancement). Légende : **[établi]** = lu dans le code ou la doc citée ;
**[supposé]** = inférence ou estimation à mesurer. Bevy 0.20.0 lu dans `~/.cargo/registry/src/*/bevy_{pbr,render,animation,camera,mesh,solari}-0.20.0`.

Écart au brief : `git merge --ff-only perf/bevy-par-iter` a échoué (la branche du worktree a 3 commits que la cible n'a pas,
26 commits en retard). Les notes `docs/wip/bevy-features.md` et `bevy-solari.md` ont été lues via `git show perf/bevy-par-iter:...`.
`out/bench/battle_perf_battle.json` est absent ; seuls `battle_mem-battle.json` et `map_mem-map.json` existent (non exploités).

## 0. Constat qui change la question

- **Les 65 514 entités ne sont pas des soldats** [établi]. Les soldats n'ont aucune entité : un `SkinnedBatch` par régiment
  (+ un lot d'imposteurs par régiment, + un lot de cadavres par régiment) porte un tampon d'instances de 48 octets par soldat
  (`client/src/battle/units/regiments.rs`, `corpses.rs`, `skinned.rs::SkinnedBatch`). Crécy = 76 régiments / 13 180 soldats
  (`docs/wip/bm-perf3.md`). Les entités sont décor/végétation/murets (`docs/wip/fix-perf.md`, `ktx2-assets.md` l. 49).
- Donc l'architecture actuelle est déjà « un draw instancié par régiment », pas « une entité par soldat ». Le nombre de draws est
  faible (de l'ordre de 150 dessins couleur + 76 x cascades en ombres [supposé d'après `queue_shadow_casters` : un élément par lot et par cascade]).
  Le coût n'est donc **pas** le nombre de draws CPU mais la charge GPU (sommets, ombres, fragments).
- `submit_pending_command_buffers` ~16 ms [établi par le brief ; `fix-perf.md` l. 99 : « le coût est GPU »] : sur Metal ce système attend/encode
  la file ; c'est le reflet du temps GPU, pas de l'encodage des soldats. **Aucune mesure n'isole la part des soldats dans ces 16 ms** [établi : introuvable dans `docs/wip/`].
  Le lot 0 ci-dessous la mesure avant tout travail.

## 1. Architecture actuelle (établi)

| Élément | Où | Fait |
|---|---|---|
| Draw | `skinned.rs` `DrawSkinned` / `DrawBatch` | un `draw_indexed` instancié par lot, phase `Transparent3d` (triée, distance 0), pipeline `SpecializedMeshPipeline` maison, éclairage = code PBR de Bevy. Pas de prépasse profondeur/normales/vecteurs de mouvement pour les soldats. |
| Ombres | `queue_shadow_casters` | chaque lot = élément `NonMesh` de la phase `Shadow` de chaque cascade du soleil : le même vertex shader d'animation rejoué par cascade ; imposteurs exclus. |
| Poses | `skinned.wesl` `vertex` → `anim_pose` → `clip_pose` → `bone_row` | **GPU**. Texture de matrices d'os cuite (`*.bones.bin`, largeur = os x 3 texels, une ligne par image de clip, 24 i/s). Par sommet : 4 os x 3 `textureLoad` x 2 images (interpolation) = jusqu'à 24 fetchs, doublé pendant un fondu de clip (`blend < 1`) ; + calcul de lance de cavalerie, variante par hash. |
| Choix du clip/temps | `anim_pose` | dans le shader, depuis le hash du soldat (`id`) et l'horloge du régiment (`anim_time`), modes Loop/Cycle/Volley/Pick. Aucun état de pose côté CPU. |
| Coût CPU par soldat | `regiments.rs::update_regiments` | reconstruit chaque image un enregistrement de 12 f32 (48 o) : position, cap, id, horloge, `far_blend`, volley, sélection ; compare aux octets précédents (`is_unchanged`). ~632 Kio/image pour 13 180 soldats (`bm-perf3.md`). |
| Envoi | `skinned.rs::prepare_buffers` | un tampon de sommets persistant, emplacements en puissances de deux, une `write_buffer_with` si ≥ 8 lots changent (`bm-perf3` : 1,39 → 0,26 ms p50). Négligeable. |
| LOD | `regiments.rs` (`LOD0_DISTANCE` 16 m, `LOD1_DISTANCE` 75 m, `IMPOSTOR_DISTANCE` 300 m, facteur `battle_lod`) | **par régiment** (distance caméra-centre), 3 maillages ~11 600 / ~1 320 / ~260 triangles (fantassin ; cavalier 15 900 / 1 950 / 584, `manifest_merged.json` `tris`), puis quad orienté caméra. |
| Culling | aucun | « aucun lot n'a de boîte englobante ; la file ne trie ni ne cull » (`bm-perf3.md`, restes ; `instancing.rs` l. 6-8). Tous les soldats de tous les lots sont habillés en sommets, vus ou non, dans la vue couleur **et** dans chaque cascade. |
| Variations | `SkinParams` (uniform par lot), masque de variantes par sommet, hash par soldat, atlas d'armes (`heraldry.rs`), atlas fins (`fine_maps.rs`) | héraldique/livrée par régiment = bind group par lot (le lot est la bonne granularité). |
| Cadavres | `corpses.rs` | lots `SkinnedBatch` en mode `Pick` (clip joué une fois, figé), mêmes shaders. |
| Campagne | `campaign/armies/skinned.rs` | mêmes lots, un par armée/famille ; quelques dizaines d'instances : hors sujet pour la performance. |

Ordre de grandeur de la charge sommet [supposé, à mesurer : la répartition réelle des LOD en bataille est inconnue] : 13 000 soldats,
dont une majorité en LOD1 (1 320 triangles ≈ 800 sommets) → ~10 M sommets habillés (≈ 24 fetchs chacun) par passe, **x (1 + cascades)**.
Le LOD0 (11 600 triangles) ne devient dominant que pour les régiments à moins de 16 m.

## 2. Options Bevy 0.20 natives et alternatives

### A. `SkinnedMesh` + `AnimationPlayer` / `AnimationGraph` par soldat — à écarter
- **Draws non regroupables** [établi] : `bevy_pbr/src/render/skin.rs::no_automatic_skin_batching` ajoute `NoAutomaticBatching` à tout `SkinnedMesh` dès que les peaux passent par un uniform buffer (`skins_use_uniform_buffers`) ; un draw par soldat, tampon de joints lié par décalage dynamique (`MAX_JOINTS = 256`, 4 matrices par unité d'allocation).
- **Entités** [établi pour la structure, supposé pour les chiffres] : une entité par os (`bevy_animation` évalue les `AnimationTarget` par entité, `par_iter_mut` aux l. 1055 et 1112 de `lib.rs`, donc parallèle mais linéaire en entités). Les rigs comptent 25-68 os (cavalerie : 68 noms dans `manifest_merged.json`). 13 000 soldats x ~30 os ≈ 400 000 entités + propagation de transformations + `extract_skins` (copie des matrices chaque image) + 13 000 draws par passe d'ombre.
- Perd aussi : mode Cycle/Volley/Pick par hash, variantes de masque, fondus pilotés shader.
- Gain attendu sur le goulot : **négatif**. Mémoire : forte. Effort : gros. Risque : élevé. Non retenu.

### B. Entités `Mesh3d` + `ExtendedMaterial` + `MeshTag` + VAT (chemin PBR natif) — peu rentable
- `MeshTag` (`bevy_mesh/src/components.rs`, lu côté shader par `get_tag(instance_index)`, `mesh_functions.wesl` l. 155) est un **u32** par instance dans `MeshUniform` : il peut indexer un tampon de stockage de variations, mais l'horloge/pose change chaque image (il faudrait de toute façon un tampon par instance, donc ce que nous avons déjà).
- Il faudrait **une entité par soldat** (13 000 `Transform` écrits chaque image, extraction `extract_meshes`, entrée `MeshInputUniform` : coût CPU/extraction à l'échelle de ce que les 65 k entités de décor coûtent déjà [supposé : plusieurs ms]) pour obtenir : culling par instance (`GpuPreprocessingMode::Culling`, si le GPU l'accepte : `INDIRECT_FIRST_INSTANCE | IMMEDIATES` + 10 buffers de stockage, `bevy_render/src/batching/gpu_preprocessing.rs` l. 1563-1600 ; **le support Metal/M4 Pro n'est pas vérifié** [supposé], regroupement automatique par (maillage, matériau), ombres et prépasse natifs, vecteurs de mouvement.
- Le regroupement automatique exige le même matériau : l'héraldique par régiment impose un matériau (bind group) par régiment, soit 76 groupes comme aujourd'hui. Le gain de batching est donc nul ; reste le culling par instance, que l'on peut obtenir sans entités (option C).
- Gain : moyen (culling, ombres) pour un coût CPU et une refonte lourds. Effort : gros. Non retenu tant que C n'est pas épuisé.

### C. Culling + LOD par soldat, dans notre pipeline instancié (recommandé, en deux paliers)
Idée : garder l'architecture (lots instanciés, animation GPU) et réduire le nombre de sommets habillés.
- **C1, côté CPU (petit/moyen).** `update_regiments` écrit déjà tous les enregistrements chaque image (boucle `for (i, pose) in poses`). On y ajoute : (a) un test sphère/frustum de la caméra par soldat pour la vue couleur ; (b) une répartition par soldat en trois maillages LOD (distance par soldat, pas par régiment) → jusqu'à 3 lots par régiment au lieu d'1 (même `params`, `Arc` partagé). Coût CPU : 13 000 tests ≈ < 0,2 ms [supposé] et nos systèmes sont mesurés < 0,5 ms.
  - Ombres : un soldat hors champ peut projeter une ombre visible. Il faut un second jeu d'enregistrements (ou une boîte de culling élargie dans la direction du soleil) pour les cascades ; le plus simple : un lot « ombre » séparé qui garde les soldats dans le volume des cascades (donc plus gros que le frustum) et utilise le LOD le plus bas (l'ombre de loin ne montre pas les détails).
  - Gain attendu [supposé] : en vue rapprochée ou inclinée, 50-90 % des soldats sont hors champ → autant de sommets en moins en couleur ; en vue d'ensemble de tout le champ de bataille, peu. LOD par soldat : un régiment étiré en profondeur n'a plus tous ses rangs au LOD du centre.
  - Risque visuel : popping de LOD (déjà géré par hystérésis par régiment ; à reprendre par soldat) ; ordre des enregistrements sans importance (l'identité est dans le champ `id`, donc les hash restent stables).
- **C2, côté GPU (moyen/gros).** Passe de calcul : culling frustum + LOD + (ombres par cascade) → tampons compactés + `draw_indexed_indirect` (compteur par atomique). Conserve les enregistrements non filtrés en mémoire GPU, supprime la dépendance au CPU. N'utilise pas `GpuPreprocessing` de Bevy (réservé aux `Mesh3d`) ; il faut un nœud de calcul maison (le dépôt a déjà `RenderCommand` maison, donc faisable). Gain additionnel par rapport à C1 : surtout si le CPU n'est pas le goulot, ce qui est déjà le cas ; **à n'entreprendre que si C1 + E ne suffisent pas**.

### D. Textures d'animation de sommets (VAT) vraies — gain réel mais coût mémoire/outil élevé
- État actuel : texture de **matrices d'os** (quasi-VAT) ; coût sommet = fetchs d'os (24, voire 48 pendant un fondu). Une VAT de positions/normales ramènerait ça à 2-4 fetchs par sommet.
- Mémoire [supposé, calculé de tête] : sommets x images x 8 o (RGBA16F). Fantassin LOD1 ~800 sommets x ~2 000 images (tous clips) ≈ 13 Mo par figure et par LOD ; 33 figures → ~0,4 Go rien qu'en LOD1 ; cavalerie (~1 500 images, 68 os) encore plus. Pour LOD2 (~150 sommets) ~2 Mo par figure : acceptable. L'empreinte actuelle est 6,5 Go (`ktx2-assets.md` l. 48).
- Perd : lance de cavalerie (déformation procédurale avant skinning, `skinned.wesl` l. 316-346), variations de cycle qui reposent sur la combinaison des os, fondu entre deux clips doit se faire par mélange de positions (faisable, deux lectures).
- Compatibilité Solari : nulle (hors TLAS de toute façon). Compatibilité héraldique/cadavres : bonne (le maillage garde ses UV/masques).
- Gain attendu [supposé] : uniquement si le temps GPU est limité par les sommets. **Hybride réaliste : VAT pour LOD2 seulement, os pour LOD0/LOD1.** Effort : gros (outil de cuisson Python/Rust, nouveau chemin shader, vérification visuelle). Reporter ; préférer E qui obtient une partie du gain sans nouveau format.

### E. Réduire le coût d'habillage par niveau de détail (petit)
- Au LOD2 (et dans les cascades d'ombres) : pas d'interpolation entre images (`f` ignoré, un seul `bone_row` au lieu de deux), pas de fondu de clip précédent (`anim_pose` appelée une seule fois), pas de lance procédurale, 1-2 os par sommet. Divise les fetchs par 2 à 4 [supposé]. Aucun format nouveau ; paramétrable via `SkinParams` (déjà un uniform par lot) ou une constante de pipeline par variante de shader (`shader_defs`), à choisir.
- Les ombres lointaines n'ont pas besoin des fondus : la variante « ombre » du vertex shader peut être allégée sans toucher à la vue couleur.
- Risque visuel : léger saccade du pas d'animation à plus de 75 m (24 i/s nominal). Effort : petit.

### F. `VisibilityRange` (LOD par distance, fondu tramé) — inadapté aux soldats, utile ailleurs
- S'applique à des entités `Mesh3d` (`bevy_camera/src/visibility/range.rs`, `check_visibility_ranges`). Nos lots n'ont pas de position propre ni de boîte ; les soldats ne sont pas des entités. Ne réduit pas la charge sommet de nos lots.
- Pourrait servir au tramé des changements de LOD (anti-popping) ou aux 1-2 lots d'armées de campagne ; gain nul sur le goulot.

### G. Imposteurs plus tôt / de meilleure qualité
- Exist déjà (quad à 300 m, `IMPOSTOR_DISTANCE`, `battle_lod` multiplie le seuil). **Levier gratuit à mesurer d'abord** : `CENT_ANS_BATTLE_LOD=0.55` (imposteurs dès ~165 m) contre 1,0 donne la borne haute du gain des LOD sans écrire de code. Un imposteur animé pré-rendu (octaédrique) est un gros chantier au risque visuel élevé : non proposé.

### H. Culling par occlusion de Bevy
- Expérimental, dépend de la prépasse profondeur et de `GpuPreprocessing` (`OcclusionCulling`, `gpu_preprocess.rs` l. 563) pour les `Mesh3d`. Nos soldats sont dans `Transparent3d` sans prépasse : **inapplicable** sans refonte. Un champ de bataille a peu d'occulteurs (relief) : gain faible [supposé]. Non retenu.

### I. Solari (`docs/wip/bevy-solari.md`)
- Seules les entités `RaytracingMesh3d` + `StandardMaterial` entrent dans la TLAS ; les soldats en sont déjà exclus [établi]. Rien dans `bevy_solari-0.20.0/src/scene/` ne mentionne le skinning ni la reconstruction de BLAS animée [établi par recherche de mots-clés vide ; l'absence de support est donc **supposée**]. Aucune des options ci-dessus n'est incompatible avec Solari ; A et B (entités `Mesh3d`) ne le deviendraient compatibles qu'avec une BLAS par pose : irréaliste.

## 3. Tableau de synthèse

| Option | Gain sur le goulot (rendu GPU) | Mémoire | Variations (héraldique, 3 LOD, cadavres) | Solari | Effort | Risque |
|---|---|---|---|---|---|---|
| A SkinnedMesh/AnimationPlayer | négatif | +++ (400 k entités) | cassées (modes shader) | non | gros | élevé |
| B Mesh3d + MeshTag + VAT | moyen (culling) | ++ | possible, 76 matériaux | non | gros | moyen |
| C1 culling + LOD par soldat (CPU) | moyen à fort en vue rapprochée [supposé] | 0 | conservées (ombres : lot dédié) | neutre | petit/moyen | faible (popping d'ombre) |
| C2 idem en calcul GPU + indirect | C1 + libère le CPU | + tampons | conservées | neutre | moyen/gros | moyen |
| D VAT LOD2 (ou tout) | moyen si limité sommets [supposé] | + (jusqu'à 0,4 Go) | lance perdue, reste ok | neutre | gros | moyen |
| E habillage allégé par LOD/ombres | petit à moyen [supposé] | 0 | conservées | neutre | petit | faible |
| F VisibilityRange | nul | 0 | n/a | n/a | petit | nul |
| G imposteurs plus tôt (réglage) | selon mesure | 0 | conservées | neutre | nul (env) | visuel léger |
| H occlusion Bevy | faible | + | inapplicable | n/a | gros | élevé |

## 4. Recommandation et plan en lots

Principe : **ne pas migrer vers les composants Bevy** (A, B) ; notre pipeline de lots instanciés avec animation par texture d'os est l'équivalent d'un rendu de foule piloté par le GPU et la voie « native » est plus coûteuse. Le gisement est la **réduction du travail de sommets** (culling, LOD par soldat, habillage allégé) et il doit d'abord être **mesuré**.

- **Lot 0 : mesurer la part des soldats (aucun code, 1 h).** Banc `tools/bevy_bench/bench.sh <étiquette> --bench-battle --offscreen` avec, tour à tour : référence ; `CENT_ANS_NO_FIGURE_SHADOWS=1` (coût des ombres des soldats, hook existant `skinned.rs`) ; `CENT_ANS_BATTLE_LOD=0.55` (borne haute des LOD) ; `CENT_ANS_BATTLE_LOD=2` (borne basse). Lire dans le dump les sections `skinned_upload` et `battle_lighting` (`shadow_casters`, `shadow_draws_total`) et `stats.lod` (répartition réelle des soldats par LOD, déjà calculée dans `update_regiments`). Critère d'arrêt : si les soldats pèsent < 2 ms GPU de 23,7, ne pas poursuivre (le goulot est le terrain, la végétation, les ombres du soleil, cf. `fix-perf.md`).
- **Lot 1 : E (habillage allégé), prototype minimal.** Variante du vertex shader pour LOD2 et pour la passe d'ombres (une image, un pas de clip, pas de fondu, pas de lance) ; mesurer l'écart au banc, comparer deux dumps. Retour arrière par constante.
- **Lot 2 : C1 (culling frustum par soldat en couleur).** Dans `update_regiments`, ne pas écrire les soldats hors champ (marge pour les ombres : voir lot 3) ; compteur de soldats dessinés/cullés dans le dump. Test unitaire du filtre (pur, sans GPU). Mesure : vues rapprochée, d'ensemble et inclinée du banc.
- **Lot 3 : ombres par lot dédié + LOD par soldat.** Lot « ombre » (soldats dans le volume des cascades, LOD bas) et partition des enregistrements en 3 lots LOD par régiment avec hystérésis par soldat. Vérification visuelle par la session principale (popping des ombres et des LOD ; pas de capture par le sous-agent).
- **Lot 4 (conditionnel) : C2 ou D.** Seulement si les lots 1-3 laissent les soldats parmi les premiers postes GPU. D limité à LOD2 (VAT hybride) ; C2 si le CPU de `update_regiments` devient visible. Chacun derrière un réglage `data/` ou une feature Cargo, désactivé par défaut tant que le banc ne montre pas de gain net (règle du chantier).

Points ouverts / à trancher par l'utilisateur ou la session principale :
1. Le GPU M4 Pro active-t-il `GpuPreprocessingMode::Culling` (info_once dans le log de démarrage : « fully supported / limited ») ? À lire dans un log existant, utile seulement si l'option B est reconsidérée.
2. Répartition réelle des LOD en mêlée de Crécy (inconnue) : lot 0.
3. Acceptabilité visuelle d'un pas d'animation sans interpolation à > 75 m (lot 1).
4. Les ombres des soldats hors champ : volume de culling et LOD d'ombre (lot 3), arbitrage visuel à valider.
