# ZG7a — perf et finitions visuelles de la vue rapprochée (ADR 0036)

Worktree d'agent `worktree-agent-a81b596dc59312952` (depuis `main` 369bc6e7). Liens symboliques non
versionnés : `data/map/pyramid`, `tools/geo/raw` → dépôt principal. Dylib :
`CARGO_TARGET_DIR=/Users/jean_hubert/dev/game_project/core/target cargo build -p godot-bridge` puis copie
dans `game/bin/libcent_ans.debug.dylib`. Rendu et perf seulement, rien dans `core/`.
Lot voisin ZG7b (export, message cache absent, `docs/geo.md`, crédits) : ne pas y toucher.

## Périmètre
1. Perf : MultiMesh par ville (ZG6, appels de dessin ×3), coût du parcellaire ZG5b et du relief ZG8,
   pic de bascule des ponts, p99 des bancs de descente (cible ≈ 115 ms, aucune tâche > 8 ms).
2. Lit fin de la Seine trop large (chenal brun) : largeurs réalistes (≈ 150-200 m à Paris).
3. Ponts-portes encore exagérés : vérifier après ZG4b.
4. Rives basses de Londres à 2-5 m au-dessus de la Tamise (au lieu du plancher 0,5 m).
5. `PathPreview` fin aux paliers vallée / site.

## État
- [x] Squelette (wip)
- [x] 1a. villes : un MultiMesh par modèle et par ville, HLOD par instance dans le shader
  (`lod_mode`, `TownBuilder.set_lod_view`) : appels de dessin de la descente villes 971 → 404
- [x] 1b. banc GPU A/B `tests/zg7a_gpu_ab.gd` (Vulkan) : parcellaire 1-2,7 ms GPU, relief ZG8 ≈ 0 ;
  parcellaire allégé (hash22, bruit fin conditionnel, finage) : −20 % de son coût
- [x] 1c. ponts-portes et ponts fins préparés dans des fils (`BridgeMeshes.build_arrays`) :
  installation d'une tuile fine 16,8 → 1,4 ms, bascule des ponts 0,3 ms par ouvrage
- [x] 1d. quadtree : image + mipmaps des pages dans un fil ; `finest_levels` (un parcours) au lieu
  d'un instantané par tuile ; éviction moins chère ; minuteries par étape (`qt_step_ms_max`)
- [ ] 1e. comparaison base (main) / ZG7a alternée, plusieurs tours (copie de `game/` à la base)
- [ ] 2. lit de la Seine
- [ ] 3. ponts-portes
- [ ] 4. rives de Londres
- [ ] 5. PathPreview
- [ ] captures `docs/img/zg7a/`, docs, addendum ADR, fusion de main, tests

## Mesures en cours (machine chargée, charge 35-100)
- Descente `--bench-towns` : appels de dessin 971 → 404.
- `fine_update_ms_max` 17,7 → 4,7 ; `fine_install_ms_max` 16,8 → 1,4.
- Quadtree (pire, sous charge 60) : sélection 9-10 ms, application 6-7 ms (GDScript, ZG2) : non traités.

## Prochaine étape
Items 2-5 (visuels), puis comparaison base / ZG7a.
