# Contrat commun des deux clients (Bevy, Godot)

Source unique : `scene.json` v2 (généré par `make_world.py`, ne pas l'éditer à la main) et les
fichiers binaires qu'il référence (voir l'en-tête de `make_world.py` pour leur format exact).
Axes : Y en haut, glTF ; la caméra regarde vers −Z.

## Scène
- `placements` : glTF entier (`node` vide) ou un nœud nommé ; `pos`, `yaw` (degrés autour de Y), `scale`.
- Terrain : maillage de la heightmap `grid × grid` sur `size_m`, centré sur l'origine, texture
  `grass_path_2` (diffuse, normale GL, rugosité) répétée tous les `uv_tile_m` mètres.
- Armée : `army.f32`, mesh = `army.models[model_index]` (glTF entier), instancié GPU.
  **Marche procédurale en vertex shader, identique dans les deux moteurs** :
  `phase = fract(sin(instance_index * 12.9898) * 43758.5453) * 2π`,
  `s = sin(2π · walk.hz · t + phase)`, translation Y `walk.bob_m · |s|`,
  rotation autour de l'axe Z local `walk.sway_deg · s` (pivot aux pieds).
- Végétation : `grass.f32` et `firs.f32`, mesh = nœud `vegetation.<x>.node`, instancié GPU ;
  seules les `floor(count · fraction)` premières instances sont dessinées.
- Caméra : interpolation linéaire de `pos` et `look_at` entre les clés de `camera_path`
  (temps `t` en secondes depuis la fin de la chauffe) ; `camera.fov_y` vertical ; far 2000 m.
- Soleil : comme `godot/proto_shot.gd` (élévation, azimut depuis −Z vers +X, couleur).
- Panneau parchemin (`ui_panel`) : panneau en haut à droite, fond parchemin (couleur
  `#e8d9b5`, bordure brune `#6b4a2b`), titre, corps, bouton ; infobulle au survol du bouton.
- Fenêtre `bench.resolution`, vsync coupée.

## Qualité (`quality`)
- `comparable` (mesures de vitesse) : ombres du soleil 4 cascades jusqu'à 150 m, SSAO, bloom
  léger, tonemapping AgX, exposition fixe, brouillard de distance (`fog.density`, `fog.color`),
  **pas** de GI dynamique ni de brouillard volumétrique, MSAA coupé, FXAA ou équivalent.
- `best` (jugement du rendu) : le meilleur que le moteur offre sans asset nouveau (Godot : SDFGI,
  SSIL, brouillard volumétrique, ciel physique ; Bevy : atmosphère, brouillard volumétrique, SSAO,
  TAA, ce qui existe en 0.20).

## Ligne de commande (identique)
- `--set clé.pointée=valeur` : surcharge de `scene.json` (valeur lue en JSON, sinon chaîne),
  ex. `--set vegetation.grass.fraction=0.5 --set quality=best`.
- `--metrics fichier.csv` : en-tête `frame,t_s,frame_ms,cpu_ms,gpu_ms,draw_calls` ; une ligne
  par image de la fin de la chauffe (`bench.warmup_frames`) à `bench.duration_s` ; champ vide si
  la mesure n'existe pas ; puis sortie du processus (code 0).
- `--shot fichier.png --at t` : capture à la première image où `t_path ≥ t` ; sortie si ni
  `--metrics` ni `--dump-state` n'attendent encore.
- `--dump-state fichier.json` : après la chauffe, écrit
  `{"engine","quality","counts":{"placements","army","grass","firs","entities_or_nodes","draw_calls"},"effective":{<valeurs de scene.json après --set>},"frame_ms_avg"}`
  puis sortie.
- Sans option : tourne jusqu'à fermeture (usage humain).

## Focus du joueur
Aucune fenêtre au premier plan. Godot : `tools/godot_bg.sh` (fenêtré) ; Bevy : `bevy/run_bg.sh`
(bundle `.app` minimal lancé par `open -g -W`, journal sur stdout). Vérifier avant/après avec
`lsappinfo front` que l'application au premier plan n'a pas changé. Jamais d'`osascript`.
