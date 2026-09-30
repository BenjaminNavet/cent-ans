extends SceneTree

## Pavois plantés devant le front : un régiment d'arbalétriers qui pivote sur place (déploiement)
## voit sa dernière rangée replantée devant son nouveau front (mêmes instances), pas laissée dans
## son dos ; un petit ajustement ne la déplace pas. (Le rendu headless ne garde pas les
## transformations des MultiMesh : on vérifie l'état du plant.)
## Usage : godot --headless --path game --script res://tests/pavise_facing_test.gd


func _initialize() -> void:
	var volleys := BattleVolleys.new()
	root.add_child(volleys)
	volleys.setup(func(_x: float, _z: float) -> float: return 0.0)
	var unit := {"id": 1, "present": true, "x": 0.0, "z": 0.0, "facing": 0.0, "width": 20.0, "depth": 4.0,
		"state": "idle", "pavise_cover": true, "side": "attacker"}
	volleys.update_fieldworks([unit])
	var count := volleys._pavises.visible_instance_count
	var rec: Dictionary = volleys._planted[1]
	var ok := count > 0 and int(rec["first"]) == 0 and int(rec["count"]) == count
	unit["facing"] = PI
	volleys.update_fieldworks([unit])
	ok = ok and volleys._pavises.visible_instance_count == count and is_equal_approx(float(rec["facing"]), PI)
	ok = ok and int(rec["rows"]) == 1
	unit["facing"] = PI + 0.2  # petit ajustement : pas de replantation
	volleys.update_fieldworks([unit])
	ok = ok and is_equal_approx(float(rec["facing"]), PI)
	unit["x"] = 30.0  # déplacement : nouvelle rangée
	volleys.update_fieldworks([unit])
	ok = ok and int(rec["rows"]) == 2 and int(rec["first"]) == count
	print("pavise_facing_test: ", "OK" if ok else "FAIL")
	quit(0 if ok else 1)
