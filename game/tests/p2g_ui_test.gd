extends SceneTree

## Chantier PO phase 2 (P2g, ADR 0097) : Techniques, Diplomatie, Cour, Fiche de personnage et
## `SaveLoadDialog` (carte et menu d'accueil) dans la zone `MODAL` de `UiLayout`.
## Squelette : désactivé.
## Usage : godot --headless --path game --script res://tests/p2g_ui_test.gd


func _init() -> void:
	await process_frame
	print("p2g_ui_test: skipped (skeleton)")
	quit(0)
