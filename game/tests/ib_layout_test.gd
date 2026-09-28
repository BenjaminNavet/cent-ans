extends SceneTree

## Chantier IB (ADR 0109), test layout — désactivé au squelette IB0 ; activé par le lot
## IB1. Voir la spec `docs/superpowers/specs/2026-09-28-ib-infobulles-design.md` § 5.
## Usage : godot --headless --path game --script res://tests/ib_layout_test.gd

const ENABLED := false


func _init() -> void:
	if not ENABLED:
		print("ib_layout_test: disabled (IB0 skeleton)")
		quit(0)
		return
	quit(0)
