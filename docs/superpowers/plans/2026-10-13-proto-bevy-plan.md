# Plan — prototype de mesure Bevy contre Godot (P0)

Spec : `docs/superpowers/specs/2026-10-13-proto-bevy-design.md`. Note d'avancement :
`docs/wip/proto-bevy.md`. **Aucun push.** Worktree `../game_project-proto-bevy`, branche
`proto-bevy`, fusion `--ff-only` dans le `main` local à la fin.

## Lot 0 — squelette (session principale)

1. Worktree + branche `proto-bevy`.
2. Restaurer depuis `a600b88f2^` les fichiers supprimés de `tools/proto_bevy/` utiles :
   `scene.json`, `terrain.py`, `make_scene.py`, `godot/project.godot`, `godot/proto_shot.gd`, `.gitignore`.
3. `scene.json` v2 : ajouter `heightmap`, `army` (nombre, rangs, espacement, modèles),
   `vegetation` (densités herbe/sapins), `camera_path` (clés temps → position/visée),
   `ui_panel` (texte, bouton, infobulle), `bench` (durée, images de chauffe).
4. Crate vide `tools/proto_bevy/bevy/` (Bevy figé à la 0.20.0,
   `[workspace]` propre pour rester hors de `core/`), `main.rs` qui parse les options
   `--metrics --shot --set --dump-state --headless-like` et sort.
5. `bench/` : projet `uv` vide (`pyproject.toml` avec la config ruff), `bench.py` qui affiche l'aide.
6. Note wip, commit `wip(proto-bevy): skeleton`.

## Vague 1 — trois agents en parallèle

### Lot A — scène commune et banc (Sonnet, mécanique)
- `make_heightmap.py` : PNG 16 bits 512×512 couvrant 1 km² ; source = le relief de bataille du
  jeu s'il existe sous forme de hauteurs exploitables, sinon `terrain.py` agrandi. Documenter la
  source dans la note wip.
- Compléter `scene.json` v2 (5 000 fantassins en 10 blocs, herbe ~200 000 brins, ~2 000 sapins,
  trajet de caméra de 20 s : vue large → vue rapprochée).
- `bench/bench.py` : lance Bevy puis Godot en alternance (≥ 3 passes chacun, Godot via
  `tools/godot_bg.sh`, Bevy via `open -g` sur un `.app` minimal ou directement si aucune fenêtre
  ne passe au premier plan), lit les CSV `--metrics`, relève `uptime` (charge), écrit
  `out/bench.csv` et un tableau p50/p95 en Markdown.
- Tests pytest : parsing CSV, calcul p50/p95, validation de `scene.json`.

### Lot B — client Bevy (agent `cent-ans-dev`, tâche de conception)
- Chargement de `scene.json` (`serde`), surcharges `--set clé=valeur` (chemin pointé).
- Terrain maillé depuis la heightmap (normales, UV, une texture `grass_path_2`).
- Placements glTF (mur, grange, meule, charrette).
- 5 000 fantassins : instanciation (batching automatique) + animation de marche. Si le skinning
  de 5 000 instances ne tient pas : animation précalculée en texture (VAT) — et le signaler, car
  le lot C devra faire de même.
- Herbe et sapins instanciés ; ombres du soleil ; brouillard ; tonemapping et exposition
  alignés sur les valeurs de `scene.json`.
- Panneau parchemin en `bevy_ui` (texte, bouton, infobulle au survol).
- Surface de contrôle : `--metrics out.csv` (temps d'image, CPU, GPU si disponible, draw calls),
  `--shot fichier.png --at t`, `--dump-state` (JSON des entités/composants/compteurs),
  `--set`, sortie automatique en fin de trajet.
- `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test` (scene.json, heightmap, `--set`).

### Lot C — client Godot (Sonnet, mécanique)
- Étendre `proto_shot.gd` en `proto_bench.gd` : même `scene.json` v2, terrain depuis la même
  heightmap, 5 000 fantassins via `MultiMeshInstance3D` (animation : même technique que le lot B),
  herbe et sapins en `MultiMesh`, panneau parchemin en `Control`.
- Mêmes options : `--metrics`, `--shot --at`, `--set`, `--dump-state` (arbre filtré + compteurs).
- Mesures : `Performance` (temps d'image, `RENDER_TOTAL_DRAW_CALLS_IN_FRAME`), temps GPU via
  `RenderingServer.viewport_get_measured_render_time_gpu`.
- Exécution uniquement via `tools/godot_bg.sh` ou `--headless` (règle du focus).

## Lot D — intégration et mesures (session principale)

1. Fusion des trois lots dans la branche ; un passage de chaque client à vide.
2. Étalonnage : scène neutre (sol gris, soleil seul), comparaison des luminances moyennes, réglage
   de l'exposition jusqu'à un écart < 5 %.
3. Banc complet (≥ 3 passes alternées), tableau p50/p95 CPU/GPU.
4. Critère 3 : un agent Sonnet par moteur reçoit la même consigne (« passer l'herbe à 50 % et le
   vérifier sans capture ») ; relever durée, tokens, nombre d'appels.
4b. Temps de recompilation incrémentale (retouche d'une couleur du panneau) : Bevy avec et sans
   `dynamic_linking`, contre le cycle Godot (modification de script + relance).
5. Critère 4 : bilan d'effort du panneau parchemin (lignes, défauts).
6. 4 captures finales (2 vues × 2 moteurs), montées côte à côte. 10 captures au plus au total.
7. ADR `0347-moteur-bevy-ou-godot.md` (numéro vérifié au moment d'écrire), index des ADR,
   note wip close, fusion `--ff-only` dans `main` local. **Pas de push.**

## Garde-fous
- Plafond : une journée d'agents ; un critère non mesuré est déclaré tel quel.
- 0 $ ; aucun asset généré ; aucune fenêtre au premier plan.
- Commits `-- <chemins explicites>`, jamais `-a`, jamais `git stash`.
