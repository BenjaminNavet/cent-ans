extends TestCase

## TW reinf (ADR 0330) : annonce « Renforts dans MM:SS » du HUD de bataille.
## Usage : godot --headless --path game --script res://tests/tw_reinf_hud_test.gd


func _init() -> void:
	await process_frame
	check(BattleHud.reinforcement_text(-1.0) == "", "rien d'attendu : pas de texte")
	check(BattleHud.reinforcement_text(0.0) == "Renforts dans 00:00", "à zéro")
	check(BattleHud.reinforcement_text(125.2) == "Renforts dans 02:06", "arrondi au-dessus")
	finish()
