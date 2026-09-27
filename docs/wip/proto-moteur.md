# Prototype comparatif Godot / Unreal (proto-moteur)

Question du joueur (27/09) : « migrer sur un autre moteur rendrait-il le jeu plus beau ? »
Réponse : prototype sur une toute petite section, mêmes assets et même caméra dans les deux
moteurs, chacun poussé au maximum. Unreal est piloté par agent (MCP `runreal/unreal-mcp`, protocole
Python Remote Execution).

## Fichiers
- `tools/proto_moteur/` : `terrain.py` (relief partagé), `make_ground.py` (Blender → `assets/ground.glb`),
  `make_scene.py` → `scene.json` (placements, caméra, soleil, brouillard), `fetch_textures.sh`.
- Projet Unreal hors dépôt : `~/dev/cent_ans_ue_proto/` (UE 5.7, Lumen, Nanite, VSM, Metal SM6,
  remote execution activée). MCP enregistré en portée locale (`claude mcp add unreal`), actif à la
  prochaine session.

## État : TERMINÉ 27/09
- [x] Terrain et scène partagés (240 m, relief doux, texture Poly Haven `grass_path_2` 2k)
- [x] Modèles du jeu convertis en glTF PBR standard (`materialize.py`) : les bâtiments encodent leur
  matière dans l'alpha des couleurs de sommet (lu par `building_atlas.gdshader`), les figurines leurs
  couleurs dans les sommets ; sans conversion ils sortent blancs hors du jeu
- [x] Rendu Godot au maximum (`godot/proto_shot.gd` : SDFGI 6 cascades, SSIL, SSAO, brouillard
  volumétrique, AgX, ombres 8K, TAA + MSAA 2×, suréchantillonnage 2×)
- [x] Rendu Unreal (`ue/` : import Interchange + Nanite, Lumen, VSM, SkyAtmosphere, nuages
  volumétriques, brouillard volumétrique ; capture hors écran SceneCapture2D suréchantillonnée 2×)
- [x] Planche `out/compare.png` (`compare.py`)

## Pipeline
`./build_assets.sh` → `python3 sync_godot_assets.py` → `godot --headless --path godot --import` →
`godot --path godot --script res://proto_shot.gd -- $PWD/out/godot.png`.
Unreal (éditeur ouvert sur `~/dev/cent_ans_ue_proto`) : `python3 ue_exec.py ue/ue_import.py`,
`ue/ue_scene.py`, `ue/ue_capture_setup.py`, `ue/ue_capture_save.py`, puis `exr_to_png.py` (Blender).

## Verdict
À assets identiques, l'écart est faible. Unreal gagne sur le ciel (nuages volumétriques natifs), la
lumière d'ambiance bleutée dans les ombres et la netteté des ombres lointaines. Godot, plus chaud et
brumeux, tient la comparaison. La différence de teinte vient surtout de l'étalonnage, réglable des deux
côtés. Aucun des deux moteurs n'améliore les figurines : Nanite n'ajoute pas de détail à un modèle
pauvre. Le goulot reste la qualité des assets, pas le moteur.

## Pièges Unreal rencontrés (macOS)
- Pas de plugin `GLTFImporter` en 5.7 : Interchange importe le glTF.
- Remote execution : un socket lié à `127.0.0.1` ne reçoit pas le multicast sous macOS →
  `RemoteExecutionMulticastBindAddress=0.0.0.0` côté éditeur **et** client (vaut aussi pour le MCP
  `runreal/unreal-mcp`).
- Fenêtre de l'éditeur masquée = rien n'est dessiné (HighResShot et AutomationLibrary muets) :
  capture hors écran par `SceneCapture2D.capture_scene()` répété, puis export EXR.
- Premier lancement : ~10 min de compilation des shaders Metal SM6.

## Suites possibles (hors prototype)
Ciel nuageux volumétrique côté Godot (shader de ciel ou `FogVolume`), étalonnage par heure du jour,
et surtout des figurines plus fines : c'est là que se joue le rendu.
