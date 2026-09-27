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

## État
- [x] Terrain et scène partagés
- [ ] Rendu Unreal (import glTF, placement, lumière, capture)
- [ ] Rendu Godot au maximum (même scène)
- [ ] Comparaison côte à côte, verdict

## Prochaine étape
Script d'import Unreal via remote execution, puis scène Godot.
