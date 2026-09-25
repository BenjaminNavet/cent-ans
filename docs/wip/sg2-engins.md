# SG2 — Engins de siège animés, huile, démos d'Avignon et de Bruges

Branche `worktree-agent-a89b4bf8d49010116`. Suite de SG1 (`docs/wip/sg1-sieges.md`, ADR 0023,
section « Suite SG2 ») et L3 (ADR 0026). Captures : `docs/audit/captures/sg2/`.

## État : terminé (main fusionné, à fusionner par l'orchestrateur)
- [x] Cœur : `Unit::reload_period`, `shot::ENGINE_RELOAD` ; pont `get_units().reload /
  reload_period` ; `debug_stage_landmark_siege` (campagne + pont) ; tests
  `sim-battle/tests/sg2.rs` (tir tous les 12 s, EngineShot = ShotEvent),
  `sim-campaign/tests/sg2_landmark_demo.rs` (Avignon, Bruges : plan, garnison, chemins d'assaut).
- [x] Modèles Blender procéduraux (`tools/blender_scripts/siege_engines.py`) : trébuchet 808 tri,
  mangonneau 360, bombarde 528, bélier 712, beffroi 932 ; `manifest.json`.
- [x] `game/scripts/battle/siege_engines_fx.gd` : trébuchet (contrepoids pendulaire, verge,
  fronde qui fouette, pierre lâchée au tir du cœur, oscillation, treuil au rechargement),
  mangonneau (claquement, treuil), bombarde (recul, mantelet, éclair et fumée au lâcher) ;
  bélier (poutre pendulaire sous le faîte) et beffroi (caisse à la hauteur du mur, pont-levis)
  avec roues et tangage selon le chemin parcouru. `release()` utilisé par SG1 (pierres sur les
  murs) et BV1 (tirs sur les régiments).
- [x] Huile : son procédural `boiling_oil` (`tools/cent_ans_tools/sg2_sounds.py`, banque AU1),
  vapeur, coulures, parement mouillé et flaque (`siege_marks_fx.gd`).
- [x] Impacts : cratères (décalques) au point frappé, effacés quand le pan tombe (S1 prend le relais).
- [x] Menu « Batailles de démonstration » (`battle_demos_menu.gd`, `data/ui/battle_demos.json`) :
  rangée, Guyenne, Paris, Avignon, Bruges ; options `--siege-landmark=` / `--siege-attacker=`.
- [x] Captures (`game/tests/sg2_engines_shot.gd`) : trébuchet à mi-tir et séquence, mangonneau,
  bélier à la porte, huile (coulée, vapeur), impacts, vues d'Avignon et de Bruges.

## Points ouverts
- Pas de capture de la bombarde au tir (aucun tir pris dans la fenêtre du script).
- Le cœur n'expose pas l'avancée réelle du bélier sous son manteau : la poutre suit le rythme
  `ram_period` comme en SG1.
- Pas de LOD pour les modèles d'engins (quelques instances par bataille, < 1 k triangles).
- Avignon : dans la sonde Rust (graine 11), la porte ne tombe pas en 600 s (bélier 160 coups,
  échelles) ; à équilibrer si la démo paraît longue.
- Les deux servants d'engin restent figés (poses statiques de B1).

## Outils
- Modèles : `blender --background --python tools/blender_scripts/siege_engines.py -- game/assets/models/siege`
- Son : `uv run --project tools --with soundfile python -m cent_ans_tools.sg2_sounds`
- Captures : `godot --path game --resolution 1600x900 --script res://tests/sg2_engines_shot.gd --
  --out=<dossier> [--landmark=avignon --attacker=fac_england --prefix=sg2_avignon] [--only=treb,ram,oil,marks]`
