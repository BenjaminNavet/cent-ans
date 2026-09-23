extends SceneTree

## Smoke test headless : charge la GDExtension, joue 10 tours, vérifie la date.
## Usage : godot --headless --path game --script res://tests/smoke.gd
## Code de sortie 0 si tout passe, 1 sinon.

const EXPECTED_LABEL := "Automne 1339"
const EXPECTED_TURN := 10

var _failures: int = 0


func _init() -> void:
	_run()
	quit(1 if _failures > 0 else 0)


func _run() -> void:
	if not ClassDB.class_exists("CampaignSim"):
		_fail("CampaignSim class not registered: GDExtension not loaded (run core/build.sh)")
		return

	var campaign: CampaignSim = CampaignSim.new()
	_check(campaign.get_turn() == -1, "turn before new_campaign should be -1, got %d" % campaign.get_turn())

	campaign.new_campaign(1337)
	_check(campaign.get_turn() == 0, "initial turn should be 0, got %d" % campaign.get_turn())
	_check(campaign.get_date_label() == "Printemps 1337", "initial date should be 'Printemps 1337', got '%s'" % campaign.get_date_label())

	for _i in range(EXPECTED_TURN):
		campaign.end_turn()

	var turn := campaign.get_turn()
	var label := campaign.get_date_label()
	_check(turn == EXPECTED_TURN, "expected turn %d, got %d" % [EXPECTED_TURN, turn])
	_check(label == EXPECTED_LABEL, "expected '%s', got '%s'" % [EXPECTED_LABEL, label])

	if _failures == 0:
		print("smoke OK: turn %d, %s" % [turn, label])


func _check(condition: bool, message: String) -> void:
	if not condition:
		_fail(message)


func _fail(message: String) -> void:
	_failures += 1
	push_error("smoke FAIL: " + message)
	printerr("smoke FAIL: " + message)
