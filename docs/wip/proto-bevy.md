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

## Lot B (client Bevy) — FAIT
- Code : `tools/proto_bevy/bevy/` (scene, world, walk + `shaders/walk*.wesl`, view, ui, mips, driver, `run_bg.sh`). fmt, clippy `-D warnings`, 13 tests verts.
- Instanciation : batching automatique de Bevy (mêmes Mesh3d + matériau, GPU preprocessing + multi-draw indirect), une entité par instance, `MeshTag` = indice du soldat pour la phase de marche. ~30 lots de dessin (opaque + alpha-mask, hors ombres/prépasse). Le coût est CPU (245 k entités) : ~17 ms/image sans herbe ni sapins, ~50-80 ms avec les 202 k instances de végétation (l'armée seule ne coûte rien depuis que le temps de marche vient de `globals.time` : écrire le matériau chaque image re-mettait en file les 35 k entités). Une vraie instanciation GPU à une entité par lot demanderait un pipeline de rendu personnalisé (hors périmètre, non éclairé/ombré dans l'exemple officiel).
- Métriques : `frame_ms`, `cpu_ms` (max du temps monde principal et monde de rendu), `draw_calls` (lots, voir ci-dessus) remplis ; `gpu_ms` vide (Bevy n'a les horodatages GPU que sous Vulkan/DX12 ; `--gpu-timing` active le plugin, mais Metal a échoué à allouer le compteur : DeviceLost).
- Écarts : glTF `fir_tree_01` (`texCoord:-1`) corrigé dans une copie `bevy/target/patched` ; terrain en 8x8 maillages ; la chauffe attend aussi la fin de compilation des pipelines ; mipmaps générées sur CPU (Bevy n'en fait pas) ; mouvement des soldats absent des vecteurs de mouvement TAA (preset best) ; `--shot` : la capture bloque l'image ~2 s (hors mesures) ; option `--set vegetation.<x>.cast_shadows=false` propre à Bevy.
- Focus : 8 exécutions fenêtrées via `run_bg.sh`, `lsappinfo front` avant/après : jamais ProtoBevy (applications au premier plan = celles du joueur : Safari, Code, Slack, Zotero...).
- Builds : release complet 1 min 26 s ; incrémental (1 ligne) 3,3 s ; `dev-fast` : premier build 59 s, incrémental 2,7 s.
- Reste : préréglage `best` tourne (~85 ms/image médiane) mais exposition non jugée à l'œil ; pas de test de l'infobulle au survol (pas de curseur en bench).
- Suite lot B (instanciation GPU personnalisée) : `bevy/src/instancing.rs` + `shaders/instanced.wesl`. Herbe et sapins = une entité par (maillage, matériau) avec le tampon brut des enregistrements en attributs d'instance, un seul dessin instancié dans la phase `Transparent3d`, éclairé par le code PBR de Bevy (soleil, ombres reçues, ambiant, brouillard). Défaut `instancing=custom` ; ancien chemin : `--set instancing=entities`. L'armée reste en entités (marche). Perdu : ombres portées, prépasse profondeur/normales (donc pas de SSAO sur ces maillages), normal maps, culling par instance (tout le lot est dessiné, comme un MultiMesh). `--dump-state` : `counts.instances` (5000/200000/2000) et `entities_or_nodes` réel (37 185 en custom, 245 179 en entités). Mesure (1 passe chacun, machine chargée) : p50 frame_ms 62 ms en custom contre 71 ms en entités ; le custom est limité par le GPU (150 M sommets d'herbe par image).

## Lot D — intégration et mesures (session principale)
- Banc n° 1 (13/10 ~16 h, `comparable`, ombres de végétation coupées dans les deux moteurs,
  3 passes alternées, 1600×900 des deux côtés) : **Bevy p50 62,1 ms / p95 67,0** (31 draw calls) ;
  **Godot p50 100,8 / p95 102,4** (93 draw calls). gpu_ms indisponible des deux côtés (Metal).
  Les deux semblent limités par le GPU (200 000 touffes × 754 sommets sans culling).
- Captures n° 1 : brouillard v1 (0,012) noyait la vue large → 0,002 ; Bevy : texte du panneau
  absent, ciel noir en `best` ; Godot : `best` à t=20 noir. Étalonnage en cours (lots B et C repris).
- Captures lues : 2/10.
