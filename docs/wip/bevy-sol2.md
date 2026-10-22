# bevy-sol2 : sol de bataille/campagne, gardes, mémoire, Solari

Branche worktree-agent-a72cb54e1aeae4ed3 (merge de perf/bevy-par-iter). Étude : `docs/wip/bevy-etude-sol.md` (branche worktree-agent-a3402a973605f20d9).

## S1 gardes de distance (fait, 990f72a4c + ground.wesl)
- Bataille (`ground.wesl`) : `normals_on = detail.x > 0 && view_dist < detail.y` ; les prises de normale par couche, du détail proche et du grain micro sont sautées quand la force des normales vaut 0 (sortie identique : elles étaient multipliées par 0). `textureSampleGrad` explicite, légal hors flot uniforme.
- Campagne (`terrain.wesl`) : `sample_layer` reçoit `detail` (albedo) et `fade` (normales) ; prises sautées quand le terme vaut 0 (sortie identique : `mix(1, ratio, 0)` et `dn * 0`) ; grain micro sauté si `micro_k <= 0`.
- Pas de test naga pour la campagne (shader de 2400 lignes avec imports Bevy) : validé à l'exécution (voir plus bas, pas de campagne lancée, seulement la bataille ; le shader campagne compile à la création du pipeline : NON vérifié en exécution).

## S2 mémoire (fait)
- `drop_cpu_copy` : après conversion en tableau mip-mappé, `asset_usage = RENDER_WORLD` pour les tableaux du sol de bataille (albedo, normales, grain micro albedo + normales). Dump `textures` : `textures/battle/tx*` cpu 0,0 Mio (gpu 73,7 Mio) ; `textures/terrain*` cpu 0,0.
- `strip_to_array` est partagé avec `siege/backdrop.rs` et `towns/atlas` : non modifié, ces images gardent leur copie CPU.
- `ImageLoaderSettings::array_layout` : non adopté. Il empile la planche en tableau sans créer de mips ; la chaîne de mips par couche (`mip_chain`) et la grille du grain micro (ordre des cellules) restent nécessaires, gain = une copie transitoire. Laissé tel quel.

## S3 sol différé pour Solari (fait)
- `ground.wesl` : `ground_pbr_input(in, is_front) -> PbrInput` (calcul commun) ; `fragment` finit par `deferred_output` (`@if(PREPASS_PIPELINE)`) ou `apply_pbr_lighting` + post-traitement (forward). `GroundExtension::deferred_fragment_shader()` pointe le même fichier.
- `solari.rs` : `force_forward::<GroundMaterial>` retiré (les autres matériaux maison restent forcés en forward).
- G-buffer : base_color (sRGB 8 bits), rugosité perceptuelle (8 bits), normale (octaédrique), émissif (rgb9e5) : tout ce que le shader produit y passe (neige, boue, splat, décals sont déjà dans base_color/roughness/N). Perdu : précision (8 bits sur la couleur/rugosité, bandes possibles dans les sols sombres) ; pas de clearcoat/spéculaire coloré (non utilisés). Le brouillard et l'ombre solaire sont appliqués par la passe d'éclairage différée / Solari.
- Prépasse : en différé le G-buffer fournit profondeur et normales (normale géométrique du sommet dans la texture de normales de prépasse) ; la normale par couche est dans le G-buffer.
- Dump : section `solari` : `{active, ground_deferred}`.
- Tests : `ground_shader_validates` (forward) et `ground_shader_validates_deferred` (naga, variantes résolues par `resolve_path`).
- Exécution (debug, offscreen, 400 images) : Solari enabled:true : exit 0, 0 erreur, dump `solari = {active: true, ground_deferred: true}`. Un avertissement « Too many textures in mesh pipeline view layout » (Solari, déjà là).

## Blocage préexistant (pas causé par ce lot)
Solari enabled:false (chemin forward) : `Too many bindings of type Samplers in Stage FRAGMENT, limit 16, count 17` sur `pbr_opaque_mesh_pipeline`, quitte l'appli (exit 1). Reproduit à l'identique sur c9aa9ac3e avec mes fichiers restaurés : donc antérieur. Le matériau fautif n'est pas identifié (la couche de vue forward compte ~12 échantillonneurs : 4 d'ombres, dt_lut, transmission, atmosphère, area_light, dfg, env map, irradiance, décals). Le chemin forward du sol n'est validé que par naga, pas à l'exécution.

## Reste
- Identifier le matériau à 17 échantillonneurs en forward (sol : 3 propres ; fusionner `micro_albedo_sampler` dans `albedo_sampler` en ferait 2).
- Contrôle visuel du sol différé sous Solari (session principale).
- Lancer la campagne pour valider le shader campagne modifié à l'exécution.

## fwd-samplers (22/10)
- Décompte forward : vue = 8 (4 ombres PCSS, area_light, dfg, décals, env map) + StandardMaterial 6 + sol 3 = 17. Les décals expliquent 1 des 17 mais ne sont pas la cause seule (un autre matériau de 6 + 8 = 14 passe).
- Correctif : sol à 1 échantillonneur propre (micro grain lit avec `albedo_sampler`, même descripteur ; relief et trample lus par `bilinear_clamped` via textureLoad). 15 en forward, décals conservés. Tests naga du sol OK.
- Non vérifié à l'exécution : tout lancement Battle (Solari on ou off, y compris HEAD sans le correctif) se fige dans `wgpu command_encoder.finish -> begin_render_pass -> ioGPUResourceListAddHashValue` (CPU 100-300 %, > 10 min pour 5 images). Cause inconnue, indépendante des décals et du correctif.
- Campagne : terrain.wesl a 10 échantillonneurs propres + vue : à vérifier/réduire.
