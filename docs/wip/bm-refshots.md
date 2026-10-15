# bm-refshots : vues de référence Godot | Bevy

État : FAIT. Outil `tools/compare_views.sh` + `tools/refshots/{views.txt,godot_refshots.gd,montage.py}` ; 8 vues capturées des deux côtés, planches relues, écarts classés dans `docs/wip/bm-fidelity.md` § Référence refshots.
Usage : `GODOT_GAME=/Users/jean_hubert/dev/game_project/game tools/compare_views.sh --out <dossier hors dépôt>` (≈ 3 min Godot + 5 min Bevy ; `--only <vue>`, `--side`, `--montage-only`).
Correctif client inclus (commit à part) : MSAA de `UiCamera` aligné sur la caméra de carte en campagne, sinon `--offscreen` ne rend pas le monde.
Points ouverts : la caméra d'ouverture de bataille diffère (cible/lacet) entre les deux clients ; Bevy fenêtré (`--bevy-window`) donne du noir quand la fenêtre est couverte.
