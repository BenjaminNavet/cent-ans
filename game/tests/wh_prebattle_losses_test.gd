extends TestCase

## WH uicards top5 : texte des pertes estimées de la fenêtre d'avant-bataille.
## Usage : godot --headless --path game --script res://tests/wh_prebattle_losses_test.gd


func _init() -> void:
	await process_frame
	var dialog: GDScript = load("res://scripts/battle/pre_battle_dialog.gd")
	var forecast := {"attacker_losses_pct": 18.2, "defender_losses_pct": 41.0}
	check(dialog.losses_text(forecast, "attacker") == " · pertes estimées : vous 18 %, ennemi 41 %", dialog.losses_text(forecast, "attacker"))
	check(dialog.losses_text(forecast, "defender") == " · pertes estimées : vous 41 %, ennemi 18 %", dialog.losses_text(forecast, "defender"))
	check(dialog.losses_text({}, "attacker") == "", "no estimate, no text")
	finish()
