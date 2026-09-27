extends SceneTree

## Chantier PO (ADR 0097) : critères C1-C3 sur la tranche verticale, en headless.
## - C1 (PO1) : aucun texte d'outil visible hors mode dev (`uv run`, `res://`, `user://`, `--…`,
##   chemins de fichier, identifiants bruts en snake_case) dans les `Label` / `RichTextLabel`.
## - C2 (PO1) : zones `UiLayout` sans chevauchement à 1280×720 et 1920×1080 ; un seul occupant de
##   `SIDE_PANEL` après l'ouverture successive de la province, de la chronique et du registre.
## - C3 (PO2) : aucune taille de police sous `Caption` (14 px de base) et 4 tailles au plus dans
##   les contrôles visibles de la tranche.
## PO0 : désactivé (chaque lot active sa partie).
## Usage : godot --headless --path game --script res://tests/po_ui_test.gd


func _init() -> void:
	print("po_ui_test: désactivé (PO0)")
	quit(0)
