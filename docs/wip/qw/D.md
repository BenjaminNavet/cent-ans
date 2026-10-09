# QW-D : textures 3D en VRAM compressé

Fait (2026-10-09).
- Règle : `tools/cent_ans_tools/texture_import_rules.py` (mode=2, mipmaps, `normal_map=1` si le nom contient « normal »), périmètre `assets/models/**` et `assets/textures/water/`. Les `.import` extraits des glb sont écrits par Godot avec `compress/mode=0` en dur : la règle s'applique donc APRÈS l'import (`tools/launch.sh` corrige puis réimporte seulement si un `.import` change ; à la main : `cent-ans art models-textures-fix`).
- Contrôle : `tools/tests/test_texture_import_rules.py` (pytest, échoue si un `.import` du dépôt est hors règle ; sans `models/dn` il ne voit que les fichiers présents).
- Mesure (sonde fenêtrée, toutes les textures de modèles chargées) : `RENDER_TEXTURE_MEM_USED` 8 980 Mo -> 1 375 Mo ; ctex courants modèles+eau : 1,26 Go.
- `generate_lods=true` (2 463 `.glb.import`) : **gardé** (arbitrage 09/10). Les LOD automatiques de Godot servent de filet aux modèles sans LOD explicites ; les couper changerait le rendu de loin sans mesure de gain. À rouvrir seulement si un banc montre un coût d’import ou de mémoire.
- `.godot/imported` garde les anciens ctex (3,7 Go -> 6,0 Go) tant qu'on ne le purge pas ; un `rm -rf game/.godot/imported` + import le ramène à la taille utile.
- Suite (09/10 soir) : la règle couvre aussi les tableaux de matières (`2d_array_texture`) de `assets/textures/` ; les tableaux de **normales** passent en `compress/high_quality=true` (BC7 au lieu de BC1, audit 08 § 3.2), y compris HB (B = rugosité) et TX. Tout nouveau paquet TX y est soumis par `launch.sh` / `cent-ans art models-textures-fix` et par le test pytest. Réimport : ~7 min (encodeur BC7). Mémoire : ×2 sur ces 7 tableaux. Smoke OK.
