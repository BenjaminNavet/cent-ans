extends TestCase

## Test headless du lot RG9 (ADR 0343) : décomposition du coût d'un trajet dans l'infobulle et
## méthodes de navigation fluviale du pont.
## Usage : godot --headless --path game --script res://tests/rg9_movement_text_test.gd


func _init() -> void:
	_run()
	finish()


func _run() -> void:
	# Terrain nu : pas de décomposition.
	check_eq(ArmyMovementController.cost_breakdown_text({"terrain_cost": 300}), "", "terrain nu")
	var text := ArmyMovementController.cost_breakdown_text({
		"terrain_cost": 300, "route_saving": -90, "pass_saving": -12, "winter_extra": 20, "general_percent": 15.0,
	})
	check(text.contains("Terrain 300"), "terrain : %s" % text)
	check(text.contains("Routes −90"), "routes : %s" % text)
	check(text.contains("Cols −12"), "cols : %s" % text)
	check(text.contains("Hiver +20"), "hiver : %s" % text)
	check(text.contains("Général +15 % de mouvement"), "général : %s" % text)
	if ClassDB.class_exists("CampaignSim"):
		var sim: Object = ClassDB.instantiate("CampaignSim")
		check(sim.has_method("get_river_voyages"), "CampaignSim.get_river_voyages missing (run core/build.sh)")
		check(sim.has_method("sail_river"), "CampaignSim.sail_river missing (run core/build.sh)")
