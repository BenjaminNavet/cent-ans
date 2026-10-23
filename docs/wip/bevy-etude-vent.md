# Étude : refaire le vent (arbres, végétation) plus près de Bevy et plus performant

État (22/10) : étude par lecture seule, aucune compilation, aucun lancement, aucun changement de code. Meshlet est écarté (`docs/wip/bevy-meshlet.md`), non reproposé.

Base lue : la branche `perf/bevy-par-iter` (f6f870711), extraite par `git archive` : le `git merge --ff-only` demandé a échoué (la branche de ce worktree a 3 commits que `perf/bevy-par-iter` n'a pas : 292adf96f, 8095a68e4, 0e12c8815 ; `perf/bevy-par-iter` a 26 commits de plus). Rien n'a été fusionné ; à trancher par l'appelant. Sources Bevy : `~/.cargo/registry/src/*/bevy_pbr-0.20.0`, `bevy_mesh-0.20.0`, `bevy_render-0.20.0`. Les exemples Bevy n'ont pas été lus (absents du registre).

Légende : [E] établi par lecture du code cité ; [S] supposé / non vérifié ; [M] à mesurer.

## 1. Ce que fait le vent aujourd'hui

### 1.1 Bataille : `client/src/battle/decor/wind.rs` + `wind.wesl` [E]

- Champ de vent GLOBAL et ANALYTIQUE, calculé au sommet, aucun calcul CPU par image. Entrées : `globals.time` (uniforme global de Bevy) et la position monde du sommet. Pas d'état par instance, pas de `MeshTag`.
- Formule (`wind.wesl`) : `phase = world.x*0.21 + world.z*0.17` (longueur d'onde ≈ 30 m) ; `sway = (0.6*sin(t*speed + phase) + 0.4*sin(t*speed*2.3 + phase*1.7)) * amplitude + gust*gust_amplitude`, avec `gust = 0.5 + 0.5*sin(t*gust_speed + x*0.012 + z*0.009)` (échelle ≈ 500 m). Décalage horizontal du sommet le long de `direction_deg`, multiplié par un poids.
- Poids : mode 0 (herbe) `1 - uv.y` de la carte ; mode 1 (cime) `(clamp(position.y / height_ref))²`, `height_ref = 0.8 * hauteur d'espèce` (`trees.rs:659`), avec `position.y` LOCAL non mis à l'échelle de l'arbre (`tree.scale`). Hiver : amplitudes ×0,5 (`trees.rs:660`).
- Coût vertex : 3 `sin`, ~20 ALU, branches sur uniforme (uniformité de flux : gratuites). En mode 0 avec carte d'aplatissement : un `textureSampleLevel` PAR SOMMET de carte d'herbe (`flatten_tex`, binding 101) + hash ; c'est la seule partie non triviale. Les sommets de feuillage coûtent surtout leur fetch et `decompress_vertex`/transformation, identiques à ceux de Bevy.
- Amplitudes (`data/fx/battle_decor_look.json:74-76`) : arbres 0,05 m + rafale 0,04 m, herbe 0,09 m + 0,07 m, direction 62°. Au poids max, ≈ 9 cm en cime d'arbre : à peine visible. Le « vent » est donc surtout un coût sans gros bénéfice perçu ; le levier principal est visuel, pas perf [E sur les valeurs, S sur la perception].
- Uniformes : `WindParams` (3 vec4 = 48 o) par MATÉRIAU, pas partagé : un matériau par couple (espèce × 3 teintes) (`trees.rs:696-698`) + un par carte d'herbe (`grass.rs:310`). Les valeurs ne varient que par `mode`, `height_ref`, hiver. Le changement de réglage demande de toucher chaque matériau (`tree_kit.rs:905`, `material.into_inner()`).
- Instancing : les arbres sont des entités individuelles (écorce + couronne × 2 LOD, jusqu'à 4 entités par arbre, `trees.rs:770-800`), maillages partagés par espèce et par niveau. L'instancing automatique de Bevy (GPU preprocess) fonctionne parce que le vent n'utilise aucune donnée par instance. L'herbe est fusionnée en chunks statiques recentrés (`grass.rs:520-550`).
- LOD / coupure à distance : `VisibilityRange` à 3 niveaux (plein jusqu'à 120 m, léger jusqu'à `impostor_from_m` 300-400 m, puis impostors atlas `far_woods.rs`, tuiles de 2000 m, `NotShadowCaster`, jusqu'à 90 000 arbres). Les impostors n'ont PAS de vent (StandardMaterial). L'herbe disparaît à `fade_far_m` 300 m. Pas de coupure du vent par distance proprement dite : le niveau léger balance encore, les impostors non.
- Passes : `enable_prepass() = false` (`wind.rs`) car le prépasse par défaut ne balance pas et cachait le sol (commentaire du fichier). Ombres : `enable_shadows` reste vrai, le vertex d'ombre est celui par défaut de Bevy (`prepass_vertex_shader()` jamais surchargé) : les ombres des couronnes sont immobiles (< 9 cm, invisible). L'herbe est `NotShadowCaster`. Conséquence : feuillage et herbe sont absents du SSAO (`battle/scene/lighting.rs:443`), de la profondeur (DoF) et des vecteurs de mouvement (flou de mouvement du lot pbr, `docs/wip/bevy-pbr.md`) [E par lecture du code ; effet visuel S].
- Ce qui est « maison » : uniquement `wind.wesl` (85 lignes, copie de l'étage vertex de Bevy sans morph/skin) et la carte d'aplatissement/sang (`flatten`, fonction de jeu, pas du vent). Le reste est du Bevy standard : `ExtendedMaterial<StandardMaterial, WindExtension>` avec `MaterialExtension::vertex_shader`, `decompress_vertex` déjà utilisé (donc déjà compatible avec la compression de maillage de Bevy 0.20, voir 2.3). Risque de maintenance : la copie de l'étage vertex doit suivre Bevy à chaque mise à jour.

### 1.2 Campagne : pipeline propre [E]

- Arbres de campagne : `campaign/vegetation/render.rs`, `billboard.rs`, `billboard.wesl` (489 lignes) = pipeline de rendu PERSONNALISÉ (commandes de rendu, tampons d'instances de 40 o, réutilise `battle::units::instancing`), PAS un `Material` Bevy. Ombres : passe non-mesh dans les vues de lumière (`shadow_pass.rs`, `shadow_cast.wesl`). Modes : impostor caméra-face, roseaux, mare, modèle proche glb.
- Vent campagne : autre modèle que la bataille. `wind_gust(p)` = bruit de valeur à deux octaves qui voyage à 2,6 m/s le long de `WIND_DIR` (constante en dur `(0.98, 0.2)` dans `billboard.wesl:78`), `shiver` par arbre (graine), flexion `local.y²`, éclaircissement de teinte par rafale. La fonction `wind_gust` est DUPLIQUÉE dans `shadow_cast.wesl:85-92` (« same gust ») pour que l'ombre suive la cime.
- Écarts : direction de vent en dur dans le shader (contraire à la règle « pas de donnée en dur »), différente de la direction 62° de la bataille ; `campaign/vegetation/meshes.rs` cuit déjà un poids de flexion dans l'alpha de la couleur de sommet (« A = wind sway weight »), mais les shaders utilisent `position.y²` ; le vent des bannières/fumée est un troisième réglage (`battle/units/wind.rs`, `battle_finish.json`).
- Sous-bois et touffes : `countryside/understory_layer.rs` (`StandardMaterial`, `AlphaMode::Mask`, `double_sided`) et `clutter.rs` (idem, `Mask(0.45)`) N'ONT PAS DE VENT (« Not ported », `clutter.rs:17`). Ils sont donc statiques et restent des `StandardMaterial` purs (compatibles Solari).

### 1.3 Mesures existantes [E]

Aucun interrupteur `CENT_ANS_PERF_OFF` pour le vent ; aucune mesure de son coût. Seule donnée voisine : en campagne le coût d'ombre vient des invocations de sommets dans les 4 cascades (`docs/wip/fix-perf.md`, 17 ms pour 1 347 instances lod1, corrigé par un lod2 d'ombre). Par analogie, le vertex des arbres de bataille (couronnes `CastShadow` par défaut) est probablement dominé par le nombre de sommets × cascades, pas par la formule de vent [S].

## 2. Options

### 2.1 Module WESL commun + `prepass_vertex_shader` (vent dans prépasse, ombres, mouvement)

- Faire : sortir `wind_offset(world, local, params)` dans un module `wind_common.wesl` importé par `wind.wesl` et par un `prepass_vertex_shader()` (copie de `prepass/prepass.wesl` d'après la lecture ci-dessus, avec le même décalage avant `position_world_to_clip`). `ExtendedMaterial` délègue `prepass_vertex_shader` / `deferred_vertex_shader` à l'extension (`extended_material.rs`, repli sur le base si `Default`) [E]. Les vecteurs de mouvement : la position précédente utilise `previous_world_from_local` sans balancement, erreur de l'ordre de l'amplitude (cm) par image : sans effet visible [S] ; on peut recalculer avec `t - globals.delta_time` (`bevy_render/src/globals.rs` expose `delta_time`) [E].
- Gain : cohérence (ombres qui bougent, feuillage dans SSAO/DoF/flou de mouvement, suppression du contournement `enable_prepass=false`). Perf : NÉGATIF (le prépasse ajoute un tirage alpha-mask du feuillage plus 3 sin par sommet dans les 4 cascades) [S, à mesurer].
- Effort : moyen. Risque visuel : moyen (le bug « ciel visible » historique revient si le vertex de prépasse diverge du principal). Solari : sans effet (le matériau est déjà exclu de la TLAS, voir 2.8).
- Verdict : seulement si on veut le feuillage dans SSAO/flou de mouvement ou si l'amplitude monte nettement (ombres visibles). Sinon NON.

### 2.2 Masque de flexion cuit dans la couleur de sommet (alpha)

- `tree_kit.rs` fournit déjà `ATTRIBUTE_COLOR` en `[f32;4]` (auto-ombrage cuit en RGB ; l'alpha vaut probablement 1, à vérifier) ; l'herbe aussi. Cuire l'alpha = poids de flexion hiérarchique : tronc (très bas), branches (profondeur du squelette), tiges de cartes (bout de rameau) et phase de frémissement par carte. `VERTEX_COLORS` est déjà dans `wind.wesl` (`vertex.color`).
- Gain : qualité (le vent actuel plie toute la cime uniformément selon `y²`, sans frémissement de feuilles) ; calcul : retire la division/`clamp` et le besoin de `height_ref`, donc un SEUL matériau par teinte et plus de dépendance à l'échelle locale ; mémoire : nulle (le canal existe) ; débloque la fusion de plusieurs arbres dans un maillage (2.7) puisque le poids ne dépend plus de la hauteur locale.
- Effort : petit-moyen (calcul dans le générateur de squelette). Risque visuel : faible. Compat : toutes les passes, sans changement. La campagne fait déjà ce choix (`meshes.rs`).
- Verdict : OUI, c'est le plus « près de Bevy » (attribut standard, pas de code de rendu).

### 2.3 Compression de maillage de Bevy 0.20 (`Mesh::compress_mesh(MeshCompressionArgs::regular())`)

- Nouveauté 0.20 [E] (`bevy_mesh-0.20.0/src/mesh.rs:271-310,1269`) : position Snorm16x4 relative à l'AABB, normale/tangente Snorm16x2 octaédrique, UV Unorm16x2, couleur Unorm8, indices U16 si possible. Foulée de sommet des arbres : 12 + 12 + 8 + 16 = 48 o → 8 + 4 + 4 + 4 = 20 o (-58 %). Les shaders de Bevy (`render/forward_io.wesl:59-86`, `prepass/io.wesl:68`) décompressent déjà, et `wind.wesl` appelle `decompress_vertex` : il n'y a RIEN à changer côté shader pour les arbres et l'herbe de bataille [E]. Le poids `position.y` lu après décompression reste correct (précision ≈ 0,3 mm sur 20 m).
- Gain attendu : bande passante de fetch (surtout les passes d'ombre qui relisent position+UV ×4 cascades) et mémoire GPU des maillages ; en RAM, passer à `RenderAssetUsages::RENDER_WORLD` seul après compression divise par deux la copie CPU (`tree_kit.rs:360` garde `MAIN_WORLD`, à vérifier avant : stats/tests lisent les sommets) [S]. Sur Apple M4 Pro (mémoire unifiée, tuilé) le gain en temps d'image est probablement faible [S] ; à mesurer sur banc.
- Contraintes : compression à faire AVANT l'extraction (panique sinon, doc de `compress_mesh`) ; Solari exige UV0 + tangentes + indices u32 sur les maillages de `StandardMaterial` (`docs/wip/bevy-solari.md`) → ne pas compresser ce qui entre dans la TLAS ; le pipeline personnalisé de la campagne (`billboard.rs`) a sa propre disposition de sommets : INCOMPATIBLE sans réécriture. Les fumées/horizon utilisent peut-être des matériaux avec leur propre shader : vérifier au cas par cas.
- Effort : petit (un appel dans `into_mesh` de `tree_kit.rs`, `grass.rs`, `far_woods.rs`). Risque visuel : faible (octaédrique sur normales « gonflées » du feuillage : à vérifier par comparaison de vues).
- Verdict : OUI comme prototype mesurable (le moins cher à essayer).

### 2.4 `MeshTag` pour la phase / les paramètres par instance

- `MeshTag(u32)` est lu dans le shader par `mesh[instance_index].tag` (`bevy_pbr render/mesh.rs:561,1194`), fait partie de l'uniforme d'instance (pas de buffer en plus) [E]. Précédent dans le dépôt : `battle/decor/horizon_smoke.rs:140`. Le tag est dans les données par instance, pas dans la clé de lot : il ne casse pas l'instancing automatique [S sur la clé de lot, à confirmer dans `GpuArrayBuffer`/batching de `mesh.rs`].
- Usage utile : indice d'espèce/graine → `height_ref`, raideur, phase (au lieu d'une phase tirée de la position monde, qui déphase les parties d'une même couronne de ~2 rad à 10 m) ; un seul matériau pour toutes les espèces d'une teinte (au lieu de 3 × espèces matériaux et autant d'uniformes).
- Gain : entités/draws quasi nul (les lots sont déjà par maillage) ; moins de matériaux et d'uniformes ; meilleure qualité de phase. Effort : petit-moyen. Dépend de 2.2 pour ne plus avoir besoin de `height_ref`.
- Verdict : accessoire ; à faire avec 2.2/2.5, pas seul.

### 2.5 Champ de vent global dans une ressource partagée (au lieu d'un uniforme par matériau)

- Une ressource `WindField` (direction, amplitude, rafale, force pilotée par la météo `battle_weather_fx.json`, cohérente avec bannières `battle/units/wind.rs`) copiée dans un `ShaderStorageBuffer` PARTAGÉ (`Handle<ShaderStorageBuffer>` référencé par tous les `WindExtension`, `#[storage(100, read_only)]`, `bevy_render-0.20.0/src/storage.rs` existe [E]) : une écriture change tous les matériaux, plus de `into_inner()` forcé, plus de duplication des réglages. Unifier la bataille et la campagne sur un seul champ (direction en donnée, plus de `WIND_DIR` en dur).
- Gain : cohérence, météo → vent, suppression de la duplication `wind_gust` entre `billboard.wesl` et `shadow_cast.wesl` via un import commun. Perf : nulle. Effort : moyen (la campagne a son propre pipeline : y brancher un binding demande de toucher `billboard.rs`). Risque : faible.
- Verdict : OUI pour la bataille ; campagne = lot séparé.

### 2.6 Rendre le vent visible à moindre coût (champ de rafales voyageant)

- Remplacer les sinus indépendants par la rafale à bande voyageante déjà écrite pour la campagne (`wind_gust`, bruit de valeur) ou par une petite texture de bruit répétée échantillonnée au sommet (`textureSampleLevel`, moins d'ALU que deux octaves de hash). Ajouter la modulation de teinte par rafale (déjà faite en campagne : `tint*(1+wind.z*gust)`) dans le fragment ou en couleur de sommet : vagues lumineuses visibles sur l'herbe/la forêt à toute distance, y compris sur les impostors statiques de `far_woods` (qui ne balancent pas) pour presque rien.
- Gain : surtout perceptuel (vent lisible avec 5-9 cm d'amplitude). Effort : petit-moyen. Risque visuel : moyen (goût, battle_decor_look).

### 2.7 Fusion par tuile / réduire les entités (arbres statiques fusionnés)

- Aujourd'hui jusqu'à 4 entités par arbre. Fusionner les arbres d'une tuile en un maillage (comme `grass.rs`) supprime l'essentiel des entités et du coût CPU d'extraction/mise en file ; possible avec le vent global SI le poids est cuit (2.2). Perd : la dissolution `VisibilityRange` par arbre (il faudrait un niveau par tuile), le culling par arbre, `use_aabb` par tuile. Mémoire : plus de géométrie (copies de maillage par instance).
- Effort : gros ; risque visuel : élevé (pops de LOD). Nombre d'entités d'arbres réel inconnu : `stats.lod_entities` (`trees.rs:800`) le dit [M]. Ne se justifie que si le profilage montre un goulot CPU d'extraction ; l'instancing automatique actuel est la voie « Bevy ». Verdict : NON pour l'instant.

### 2.8 Compatibilité alpha-mask / différé / Solari (point à corriger, établi par lecture)

- `SolariLightingPlugin` impose `DefaultOpaqueRendererMethod::deferred()` à toute l'application (`docs/wip/bevy-solari.md`, `battle/scene/solari.rs:4`), et `data/fx/solari.json` est maintenant `enabled: true` (commit f6f870711), mais seulement avec la feature Cargo `solari`.
- Dans `bevy_pbr-0.20.0/src/material.rs:1344-1395` et `:1738-1742`, un matériau dont la méthode résolue est `Deferred` (cas `Auto` → défaut différé, ce qu'est `StandardMaterial` par défaut et donc `WindMaterial`, `opaque_render_method` délégué au base) est ÉCARTÉ de la file de la passe principale (`continue`), pour les phases Opaque ET AlphaMask. Il n'est dessiné que par le prépasse différé, que `enable_prepass() = false` désactive pour `WindMaterial`. Conclusion [E par lecture, S à l'exécution] : sous Solari, tout le feuillage et l'herbe de bataille (WindMaterial) seraient probablement invisibles. Remède : forcer `opaque_render_method: OpaqueRendererMethod::Forward` dans le `StandardMaterial` de base de `WindMaterial` (propriété du base, `trees.rs:674`, `grass.rs:323`), ce qui est cohérent avec « les matériaux custom sont hors Solari » ; à vérifier au runtime avec `--features solari`. Aussi les `StandardMaterial` alpha-mask de campagne/sous-bois passent en différé : le `diffuse_transmission` des feuilles n'est pas garanti dans le G-buffer différé [S].
- Les maillages de vent (`ExtendedMaterial`) restent hors TLAS : ni ombres RT ni GI sur arbres. Ne pas chercher à les y mettre : l'API n'accepte que `StandardMaterial` (`docs/wip/bevy-solari.md`).

### 2.9 Morph targets, textures d'animation de sommets, vertex pulling

- Morph targets : position supplémentaire par sommet (+12 o), poids par instance mis à jour par le CPU chaque image pour des milliers d'arbres : pire que le vent analytique. NON. Textures d'animation de sommets / vent précuit : pas de gain face à 3 `sin`. NON.

### 2.10 Impostors / billboards lointains

- Aujourd'hui : atlas statique (bataille), imposteurs à vent (campagne, flexion + tremblement par arbre). Pour la bataille, ajouter un `shiver` par tuile n'a pas de sens à 300 m+ (< 9 cm) ; la modulation de teinte de 2.6 suffit. NON pour un vent géométrique.

## 3. Synthèse

| Option | Gain | Effort | Risque visuel | Ombres / prépasse / mouvement | Solari |
|---|---|---|---|---|---|
| 2.1 vent dans prépasse | cohérence, SSAO/flou ; perf négative [S] | moyen | moyen | corrige les trois | neutre |
| 2.2 masque alpha cuit | qualité (hiérarchie, frémissement), 1 matériau/teinte | petit-moyen | faible | neutre | neutre |
| 2.3 compression Bevy | -58 % foulée sommet, RAM ÷2 [M] | petit | faible | déjà géré par Bevy | à exclure des maillages TLAS |
| 2.4 MeshTag | phase par arbre, moins de matériaux | petit-moyen | faible | neutre | neutre |
| 2.5 champ global partagé | cohérence, météo, 0 duplication | moyen | faible | neutre | neutre |
| 2.6 rafales visibles | perception | petit-moyen | moyen | neutre | neutre |
| 2.7 fusion par tuile | CPU entités | gros | élevé | perd la dissolution par arbre | neutre |
| 2.8 forcer Forward | correctif Solari | petit | faible | n/a | requis |
| 2.9, 2.10 | aucun | n/a | n/a | n/a | n/a |

Constat de fond : le vent de bataille n'est ni lourd ni très « maison » ; le coût vertex des arbres vient du nombre de sommets (couronnes d'espèces ×2 LOD, ombres ×4 cascades), pas de la formule. Le gain de PERF réaliste vient de 2.3 (si le banc le confirme) et d'une réduction du nombre de sommets/passes (hors vent) ; le gain de QUALITÉ vient de 2.2 + 2.6. À ne pas promettre : « vent plus performant » sans mesure.

## 4. Recommandation et plan

Lots, chacun mesurable, aucun ne doit dégrader le banc :

1. Lot V0 (mesure, prérequis) : interrupteur `CENT_ANS_PERF_OFF=wind` (wind.wesl devient un décalage nul, ou `StandardMaterial` brut) + bascule `VisibilityRange` ; banc bataille (sandbox ultra, 3 passes alternées) pour établir le coût réel du vent, du nombre de sommets de couronnes et d'ombres, du nombre d'entités d'arbres (`stats.lod_entities`). Décision des lots suivants sur ces chiffres. Un prototype de vent sans formule (offset constant) borne le gain maximal de toute réécriture.
2. Lot V1 (prototype minimal, petit) : `compress_mesh(regular())` sur les maillages de couronne/écorce (`tree_kit.rs::into_mesh`), de l'herbe (`grass.rs::into_mesh`) et des tuiles `far_woods.rs`, derrière un réglage data (`render_quality.json`, par préréglage) ; mesurer mémoire GPU/RAM (dump), temps d'image (banc), diff visuel d'UNE vue de contrôle (`tools/compare_views.sh`). Si le gain < bruit, laisser désactivé.
3. Lot V2 (qualité, petit-moyen) : flex cuit en alpha (2.2) + `MeshTag` (2.4) + rafale voyageante et teinte (2.6), amplitudes en data ; supprime `height_ref`/`mode` du matériau. Preuve : dump + une vue de contrôle.
4. Lot V3 (moyen) : `WindField` global (2.5) avec import WESL commun, direction en donnée, couplage météo ; lot campagne séparé pour supprimer le `WIND_DIR` en dur et la duplication `billboard.wesl`/`shadow_cast.wesl`.
5. Lot V4 (conditionnel, moyen) : vent dans le prépasse (2.1) seulement si on veut le feuillage dans SSAO/flou de mouvement ; mesurer le surcoût.
6. Lot S (petit, indépendant) : `OpaqueRendererMethod::Forward` sur les matériaux de vent + test runtime `--features solari` pour confirmer/infirmer 2.8.

## 5. Points ouverts / à trancher

- Merge de la base : la branche du worktree a divergé de `perf/bevy-par-iter` (voir en tête).
- Valeur exacte de l'alpha des couleurs de sommet de `tree_kit` (supposée 1) ; tests/stats qui lisent les sommets côté CPU (impact de `RENDER_WORLD` seul).
- Le point 2.8 (feuillage invisible sous Solari) est une déduction de lecture, non exécutée.
- Exemples Bevy non consultés.
- Choix de conception à faire par l'utilisateur : rendre le vent plus visible (amplitude/teinte) ou le garder discret ; garder le vent campagne et bataille séparés ou les unifier.
