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
- [ ] Squelette (wip)
- [ ] 1. perf
- [ ] 2. lit de la Seine
- [ ] 3. ponts-portes
- [ ] 4. rives de Londres
- [ ] 5. PathPreview
- [ ] captures `docs/img/zg7a/`, docs, addendum ADR, fusion de main, tests

## Prochaine étape
Mesures de base (bancs) et lecture du code des villes (MultiMesh par cellule).
