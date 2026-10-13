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

## Prochaine étape — REPRISE (pause du 13/10, ~17 h 45)
Fait : lots 0, A, B, C, étalonnage des deux côtés, anti-occlusion des deux côtés (Bevy rendu
hors écran `9a4675069`, Godot SubViewport + `force_draw` `c02845e2a`), contrôles « scène
vraiment dessinée » (`PROTO_*_FATAL`, passe échouée dans `bench.py`). Dernier commit `c02845e2a`.

Aucun banc valide à ce jour : n° 1 (terrain Godot invisible), n° 2 (Bevy passe 1 vide, fenêtre
masquée), n° 3 (Godot passes 2-3 vides). Seule paire propre du n° 3 : Bevy 72 ms / Godot 120 ms
(charge 18) ; Bevy passes 2-3 : 62,6 / 61,8 ms.

À faire, dans l'ordre, **sur machine calme** (charge < 4 ; la seed-bots/Blender/cargo de l'autre
session faisaient monter à 20) :
1. 3 passes Godot seules pour valider l'anti-occlusion (p50 attendu ~100-125 ms ; à charge 4-9
   le `force_draw` donnait 170-265 ms : vérifier s'il coûte ou si c'était la charge — comparer
   une passe fenêtre visible).
2. `bench.py shots --engine godot`, comparer RVB haut/bas 300 lignes aux références (Lot D,
   anciennes PNG copiées dans `tools/proto_bevy/out/ref_godot/` (ignoré par git), et valeurs chiffrées ci-dessous),
   ±5 %. Les PNG Godot de `out/` sont peut-être partielles (pause en pleine capture).
3. Banc n° 4 complet : `cd tools/proto_bevy/bench && uv run bench.py run --passes 3 --quality
   comparable --set vegetation.grass.cast_shadows=false --set vegetation.firs.cast_shadows=false
   && uv run bench.py report` ; puis `shots` + `montage` (captures lues : 2/10).
4. Critère 3 : un agent Sonnet par moteur, consigne « herbe à 50 %, vérifier sans capture » ;
   relever durée, tokens, appels. Temps de recompilation Godot vs Bevy (Bevy déjà mesuré).
5. Critère 4 : effort du panneau d'interface (police : Bevy n'a qu'ASCII par défaut).
6. ADR `docs/decisions/0347-moteur-bevy-ou-godot.md` (vérifier le numéro libre) + index ;
   noter que Bevy rend hors écran sans présentation (vsync coupée, écart attendu faible).
7. Fusion `--ff-only` dans le `main` local, **jamais de push** ; prévenir le joueur.

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

## Lot C (client Godot) — anti-occlusion

- 10-13 (pause joueur) : FAIT = monde 3D dans un SubViewport (UPDATE_ALWAYS) + TextureRect, `force_draw(false, delta)` quand `window_can_draw()` est faux (seule méthode qui rend fenêtre masquée ; le SubViewport seul ne suffit pas), garde-fou `PROTO_GODOT_FATAL` (seuil 40 draw calls, normal 50-141, 69 en fin d'échauffement), `bench.py` échoue une passe si le journal contient `PROTO_*_FATAL` + `shots --engine` + test. PAS FAIT = p50 de contrôle (machine à charge 20, 525 ms mesurés, non représentatif ; en forcé masqué 170-265 ms) et comparaison RGB des 4 captures (shots godot interrompus, anciennes copies dans le scratchpad). Prochaine étape : machine calme, 3 passes puis `bench.py shots --engine godot` et delta RGB.

## Lot D — intégration et mesures (session principale)
- Banc n° 1 (13/10 ~16 h, `comparable`, ombres de végétation coupées dans les deux moteurs,
  3 passes alternées, 1600×900 des deux côtés) : **Bevy p50 62,1 ms / p95 67,0** (31 draw calls) ;
  **Godot p50 100,8 / p95 102,4** (93 draw calls). gpu_ms indisponible des deux côtés (Metal).
  Les deux semblent limités par le GPU (200 000 touffes × 754 sommets sans culling).
- Captures n° 1 : brouillard v1 (0,012) noyait la vue large → 0,002 ; Bevy : texte du panneau
  absent, ciel noir en `best` ; Godot : `best` à t=20 noir. Étalonnage en cours (lots B et C repris).
- Captures lues : 2/10.
- Calibration Bevy (suite) : police EB Garamond du dépôt chargée en octets (le texte du panneau s'affiche, ~2x plus de pixels sombres dans le panneau) ; `comparable` : exposition 12,2 EV, ambiant 1400 -> moyennes RVB des 300 lignes haute/basse à moins de 10 % de Godot à t=0 et t=20 (haut/bas t=0 : 166,5/163,9/161,1 et 155,6/146,7/139,1 contre 167,8/167,1/165,7 et 156/149,6/145,1 ; t=20 : 154,1/145/141,1 et 156,1/137,5/108 contre 151,2/141/136,9 et 153,6/131,1/99,7). `best` : exposition 11,4 EV, ciel matinal orangé (avant : haut ~[31,18,9]) ; reste plus clair que Godot au sol à t=20 (+30 %), conflit entre t=0 et t=20 sur un seul réglage. Herbe : visible mais petite (touffes de 20 cm, 2,8/m2), ~2,3 % des pixels du bas de l'image à t=20. Réglages Bevy-only : `--set bevy.ev100|ambient|sun_scale|fog_scale|bloom=...`. Les captures à t=0 pouvaient sortir noires : la capture attend maintenant 60 images.
- Bug « végétation parfois non dessinée » : cause = fenêtre occultée (recouverte par une autre appli) -> macOS ne donne pas de texture de swapchain, Bevy ne rend rien (terrain compris), image à 16 ms. Correctif : la caméra rend dans une image hors écran (`--set bevy.offscreen=false` pour la fenêtre), l'UI cible cette caméra, les captures lisent l'image ; garde-fou : la chauffe n'aboutit que si tous les lots instanciés (5) sont dessinés, sinon `PROTO_BEVY_FATAL` dans le journal et `run_bg.sh` sort en code 1 (idem « Quitting the application due to »). 5 passes : 5/5 lots, p50 65,0 / 61,4 / 73,8 / 72,1 / 72,5 ms.
