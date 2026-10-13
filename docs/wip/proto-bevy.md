# P0 — prototype de mesure Bevy contre Godot

Spec : `docs/superpowers/specs/2026-10-13-proto-bevy-design.md` ; plan :
`docs/superpowers/plans/2026-10-13-proto-bevy-plan.md`. Worktree `../game_project-proto-bevy`,
branche `proto-bevy`. **Aucun push** (consigne du joueur).

## État
- Lot 0 (squelette) : fait. Première compilation de Bevy 0.20 : 2 min 36 s (dev, deps en opt 3).
  - Dossier `tools/proto_bevy/` (et non `tools/proto_moteur/`, ignoré en bloc par `.gitignore:60`).
  - Restaurés depuis `a600b88f2^` : `scene.json` v1 (1 428 placements, caméra, soleil, brouillard),
    `terrain.py`, `make_scene.py`, `godot/project.godot`, `godot/proto_shot.gd`.
  - Assets glTF copiés depuis `tools/proto_moteur/assets` du checkout principal (ignorés par git,
    35 Mo) ; `godot/assets` idem (44 Mo).
  - Crate `bevy/` (Bevy `=0.20.0`, workspace autonome), options `--scene --metrics --shot --at
    --dump-state --set` parsées.
  - `bench/` : projet `uv` vide.

## Prochaine étape
Vague 1 : lots A (scène + banc), B (client Bevy), C (client Godot) en parallèle.

## Points ouverts
- Source de la heightmap : le relief de bataille du jeu si exploitable, sinon `terrain.py` agrandi.
  (`game/assets/horizon/relief/*.bin` = lignes d'horizon, pas des hauteurs.)

## Lot A (bench)
- `tools/proto_bevy/bench/bench.py` : `run --passes N --quality comparable [--set k=v]`, `report`, `shots`, `montage` (Pillow). Lanceurs configurables en tête de fichier. Sorties dans `tools/proto_bevy/out/` (à gitignorer côté racine).
- Testé sur faux CSV uniquement (`uv run --project tools/proto_bevy/bench pytest`). Reste : essai réel quand les lots B et C existent.

## Lot B (client Bevy)
- État : `bevy/src/scene.rs` fait (chargement `scene.json` + `--set` pointé, heightmap, enregistrements, chemin caméra ; 7 tests). Reste : rendu, matériau de marche, UI, métriques, `run_bg.sh`.
- Build release complet de Bevy 0.20 (12 cœurs) : 1 min 26 s.

## Lot C — client Godot (`tools/proto_bevy/godot/proto_bench.gd`)
- Fait : tout le contrat (terrain `ArrayMesh` 1025² avec normales/tangentes, placements, armée en
  3 `MultiMeshInstance3D` (un par modèle) avec `ShaderMaterial` de marche, herbe et sapins en
  `MultiMesh` + `visible_instance_count`, caméra, soleil, presets `comparable`/`best`, panneau
  parchemin en `Control`, `--set`, `--metrics`, `--shot/--at`, `--dump-state`, sortie auto).
- Données : `godot/assets/` (ignoré par git) contient une copie de `scene.json` v2 et des `.f32`
  (à recopier si `make_world.py` régénère). Tous les chemins de `scene.json` sont résolus par leur
  nom de fichier dans `res://assets/` ; textures `ground_grass_path_2_*` (identiques aux
  `grass_path_2_*`), `.import` de la normale passé en `normal_map=1`.
- Écarts : phase de marche = indice global dans `army.f32` (passé en `INSTANCE_CUSTOM.x`), pas
  `INSTANCE_ID` (par MultiMesh) ; `frame_ms` = horloge brute (`Time.get_ticks_usec`), le `delta`
  de Godot étant lissé sur la fréquence d'écran ; `gpu_ms` vide (Metal ne renvoie rien) ;
  `--dump-state` écrit 30 images après la chauffe, `frame_ms_avg` sur ces 30 images ; sorties en
  chemin absolu (`open` lance depuis `/`).
- Focus : `tools/godot_bg.sh` ne suffit pas, Godot devient l'application au premier plan dès le
  lancement (testé : `window/size/no_focus`, `-j`, `--minimized`, aucun n'y change rien).
- Mesure : machine chargée par les autres agents, `frame_ms` quantifié (6,9 ms ou 133-168 ms
  selon l'état) ; refaire les mesures sur machine calme.
