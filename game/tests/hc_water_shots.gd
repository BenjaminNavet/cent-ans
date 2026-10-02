extends SceneTree

## Planche de contrôle du lot HC2 (ADR 0161 §3, eaux lisibles) — squelette.
## Usage : godot --path game --resolution 640x400 --script res://tests/hc_water_shots.gd -- --out=<dossier>


func _init() -> void:
	await process_frame
	quit(0)
