# Lot vent-ombres : le vent dans la passe d'ombres et le prépasse

État (23/10) : code écrit, compilé, tests unitaires verts ; AUCUNE vérification GPU (consigne : aucun client lancé par ce lot).

## Ce qui est fait
- `client/src/battle/decor/wind_common.wesl` (bibliothèque WESL, `load_shader_library!`) : `WindParams`, `wind_displace(...)` = tout le calcul du vent (balancement, rafale, herbe couchée/sang). Importé par les deux vertex shaders : pas de copie.
- `wind.wesl` (passe principale) : réécrit au-dessus du module commun (même résultat qu'avant).
- `wind_prepass.wesl` (nouveau) : copie du vertex de `prepass.wesl` de Bevy (sans morph/skin) avec le même déplacement, `globals` au binding 1 du groupe vue (comme `camp_horse_shadow.wesl`). Position précédente (vecteurs de mouvement) calculée avec `globals.time - globals.delta_time`. Le fragment reste celui du matériau de base : `pbr_prepass` fait le `discard` de l'`AlphaMode::Mask` (arbres 0,5, herbe 0,4), rien à ajouter. Aucun sampler ajouté (le sampler `flatten` est déjà dans le groupe matériau, lu au vertex).
- `WindExtension::prepass_vertex_shader()` + `enable_prepass()` pilotés par `wind_passes` de `data/fx/battle_decor_look.json` (schéma mis à jour), lus une fois (`WindPasses::current()`, `OnceLock`) car les fonctions du trait sont statiques.
- Comment la passe d'ombres choisit son vertex (bevy_pbr 0.20 `material.rs`) : `add_shader(PrepassVertexShader, M::prepass_vertex_shader())` : le MÊME shader sert le prépasse caméra et les ombres ; seul `enable_prepass()` décide si la caméra l'utilise (`enable_shadows()` reste vrai). Les ombres posent aussi `DEPTH_PREPASS`, donc le shader ne peut pas distinguer ombre et prépasse caméra.

## Réglages (`wind_passes`) et défauts
| `wind_in_shadows` | `wind_in_prepass` | effet |
|---|---|---|
| false | false | comportement historique exact (vertex Bevy par défaut, pas de prépasse) |
| true (DÉFAUT) | false (DÉFAUT) | ombres des arbres qui balancent ; pas de feuillage dans SSAO/profondeur/flou |
| true/false | true | prépasse caméra ET ombres via le vertex balancé (on ne peut pas séparer) |

Défaut `wind_in_prepass:false` : le risque connu (SSAO + cartes d'herbe qui balancent : le prépasse des cartes immobiles masquait le sol) est corrigé en principe par le vertex commun, mais NON vérifié visuellement (aucun lancement permis). Défaut `wind_in_shadows:true` : l'ombre suit le même déplacement que le rendu (amplitude 18 cm), pas de SSAO en jeu ; coût attendu négligeable (3 sin par sommet d'ombre, déjà mesuré neutre pour le vertex principal). À confirmer par le banc.

## Tests
`look::tests::wind_passes_default_and_parse`, `wind::tests::{prepass_vertex_follows_the_settings, shipped_data_keeps_the_camera_prepass_off, wind_common_compiles_and_validates}` (WESL -> WGSL -> naga pour le module commun). Les deux vertex shaders complets ne sont pas validés par naga (ils dépendent des imports Bevy et de ses shader defs) : seule la compilation par Bevy au lancement les vérifie.

## Vérifications à lancer par la session principale (GPU)
Préparer le worktree (liens `data/map/pyramid`, rsync des assets) puis :
1. Démarrage sans erreur de shader (les erreurs de pipeline apparaissent en log) :
   `CARGO_INCREMENTAL=0 CENT_ANS_MUTE=1 CENT_ANS_FEATURES=dev CENT_ANS_PROFILE=debug client/run_bg.sh --state Battle --frames 300 --offscreen --dump-state out/vent-ombres.json` puis `grep -i "error\|wgsl\|naga" ` du log.
2. Défaut (ombres seules) : une capture de bataille avec arbres au soleil bas, deux instants (`--frames 500`, puis `--frames 560`) : l'ombre d'un arbre doit se décaler de quelques cm.
3. Prépasse : éditer temporairement `wind_passes.wind_in_prepass:true` dans `data/fx/battle_decor_look.json`, capture bataille avec SSAO + flou de mouvement actifs sur de l'herbe : le sol ne doit pas disparaître (ciel visible) ; vérifier aussi un arbre feuillu (le feuillage doit faire de l'AO, pas de carrés = discard du masque OK).
4. Coût (banc release, un seul à la fois) :
   `tools/bevy_bench/bench.sh vent-off --bench-battle --offscreen` avec `wind_passes` à false/false, puis `vent-ombres` (défaut), puis `vent-prepasse` (true/true). Comparer les ms/image.
5. Si 3 est bon et le coût neutre : passer `wind_in_prepass` à true dans le JSON.

## Prochaine étape
Retours des vérifications ci-dessus ; éventuellement des défauts différents.
