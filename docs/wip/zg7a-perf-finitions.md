# ZG7a — perf et finitions visuelles de la vue rapprochée (ADR 0036)

Worktree d'agent `worktree-agent-a81b596dc59312952` (depuis `main` 369bc6e7). Liens symboliques non
versionnés : `data/map/pyramid`, `tools/geo/raw` → dépôt principal. Dylib :
`CARGO_TARGET_DIR=/Users/jean_hubert/dev/game_project/core/target cargo build -p godot-bridge` puis copie
dans `game/bin/libcent_ans.debug.dylib`. Rendu et perf seulement, rien dans `core/`.
Lot voisin ZG7b (export, message cache absent, `docs/geo.md`, crédits) : ne pas y toucher.

## État
- [x] Squelette (wip)
- [x] 1a. villes : blocs par ville, détail par modèle et cellule de 1 km, HLOD par instance dans le shader
  (la version « détail par ville » était 9 % plus lente : abandonnée)
- [x] 1b. banc GPU A/B `tests/zg7a_gpu_ab.gd` (Vulkan) ; parcellaire allégé (−20 % de son coût)
- [x] 1c. ponts-portes et ponts fins préparés dans des fils
- [x] 1d. quadtree : image + mipmaps dans un fil ; `finest_levels` ; éviction ; minuteries
- [x] 1e. comparaison base / ZG7a alternée (copie `game/` à 369bc6e7 dans le scratchpad) : tableau dans
  `docs/godot-map.md` (section ZG7a)
- [x] 2. Seine : diagnostic — le « chenal brun » était le fond de vallée plaqué à 0,5 m (rehaussement
  de rendu E1-E4 + plancher) sans parcellaire, corrigé par ZG4b (`hc < 0.25`) ; largeurs incohérentes
  par tronçon (4,5 m en amont de Rouen, 50 m Elbeuf-Rouen, 60 m à Mantes) : `hydro_fine` interpole
  maintenant le long de la chaîne des ancrages ; ancrages La Bouille, Duclair, Caudebec ;
  `geo hydro-fine` relancé (dossier temporaire puis copie dans le cache partagé, jeu de tuiles
  identique, sauvegarde dans le scratchpad) ; pas de lit creusé dans les zones personnalisées.
- [x] 2b. `anchors-fine` relancé (largeurs des ponts), installé
- [x] 3. ponts-portes et ponts fins : tablier à sa largeur réelle (`BridgeMeshes.fine_deck_scale`)
- [x] 4. Londres : plancher monotone `max(0,5 ; min(0,85 h ; 5 m))` dans `detail_dem.apply_boost`,
  `BAKE_VERSION` 4, zone `londres` seule recuite (rives 2,6-3,8 m au lieu de 0,5) ; sauvegarde
  des tuiles d'avant dans le scratchpad. Les autres zones seront recuites au prochain
  `geo detail-dem` complet (marqueurs invalidés).
- [x] 5. `PathPreview` fin aux paliers proches (`update_view` chaque image)
- [x] captures `docs/img/zg7a/`, docs, addendum ADR, test `zg7a`
- [x] fusion de main (fecf87ec), tests Godot zg2/zg4/zg5b/zg6/zg7a/zg8/smoke OK, pytest 623 OK, ruff OK

## Découvertes hors lot
- Le relief E1-E4 (ZG1) plaque les fonds de vallée proches de plateaux à 0,5 m (rehaussement de
  rendu σ 5 km, gain 0,8, puis plancher de côte) : Seine à Vernon, Mantes, Poissy à 0,5 m au lieu de
  10-25 m, niveau d'eau fin à 0,5 m de Paris à Rouen. Double emploi avec ZG8 (exagération à
  l'exécution). À traiter par une recuisson E1-E4 (plancher monotone comme ZG7a) : hors lot.
- Tamise fine : niveau d'eau -7,8 m à Londres (PAVA mêlé à la bathymétrie de l'estuaire).

## Prochaine étape
Lot terminé ; reste la fusion dans main par l orchestrateur. Pistes : recuisson E1-E4 (plancher monotone), quadtree select/apply > 8 ms.
