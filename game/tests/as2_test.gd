extends TestCase

## Lot AS2 : cadence de marche des figurines de carte calée sur la vitesse écran, et hampe de
## l'étendard qui suit la marche / le tangage. Vérifications par valeurs, sans rendu.
## Usage : godot --headless --path game --script res://tests/as2_test.gd


func _init() -> void:
	await process_frame
	check(ArmyFigures.as2_enabled(), "AS2 actif (data/fx/campaign_army_walk.json)")
	_check_formula()
	await _check_troop()
	await _check_fleet()
	finish()


func _check_formula() -> void:
	var nominal := 1.35
	var world := ArmyFigures.FIGURE_SCALE
	check(absf(ArmyFigures.cadence_factor(nominal * world, nominal, world) - 1.0) < 0.001, "vitesse nominale → facteur 1")
	check(absf(ArmyFigures.cadence_factor(nominal * world * 0.5, nominal, world) - 0.5) < 0.001, "demi-vitesse → facteur 0,5")
	var cfg: Dictionary = ArmyFigures.walk_settings().get("cadence", {})
	check(ArmyFigures.cadence_factor(0.0, nominal, world) == float(cfg["min_factor"]), "arrêt en marche → plancher")
	check(ArmyFigures.cadence_factor(1000.0, nominal, world) == float(cfg["max_factor"]), "vitesse folle → plafond")


func _make_marker(unit_type: String, embarked: bool) -> Array:
	var marker := Node3D.new()
	root.add_child(marker)
	var army := {"faction": "fac_france", "stance": "normal", "position": Vector2(100, 100),
		"embarked": embarked, "units": [{"unit_type": unit_type, "strength": 600}, {"unit_type": "unit_longbowmen", "strength": 900}]}
	var figures := ArmyFigures.build(army, Color(0.2, 0.3, 0.8), null, "as2")
	marker.add_child(figures)
	return [marker, figures]


func _check_troop() -> void:
	var pair := _make_marker("unit_knights", false)
	var marker: Node3D = pair[0]
	var figures: ArmyFigures = pair[1]
	var nominal := float(BattleGore.settings().get("cadence", {}).get("walk", 1.35))
	var world_speed := nominal * ArmyFigures.FIGURE_SCALE * 0.5  # demi-vitesse du clip
	figures.set_walking(true)
	var dt := 1.0 / 60.0
	for i in 240:
		marker.position.x += world_speed * dt
		figures._process(dt)
	var factor := figures.group_factor("infantry_0") if figures.group_factor("infantry_0") != 1.0 else figures.group_factor("archer_0")
	check(absf(factor - 0.5) < 0.08, "marche à demi-vitesse → cadence ≈ 0,5 (obtenu %.3f)" % factor)
	check(figures.bearer_tilt() != Basis.IDENTITY, "en marche la hampe se balance")
	check(figures.bearer_dynamic(), "hampe dynamique en marche")
	# Horloge : avance moins vite que le temps réel.
	var clock_before := _clock(figures)
	for i in 60:
		marker.position.x += world_speed * dt
		figures._process(dt)
	var advanced := _clock(figures) - clock_before
	check(advanced < 0.75 and advanced > 0.3, "horloge ralentie : %.3f s pour 1 s réelle" % advanced)
	# Arrêt : retour à 1, hampe droite après le fondu.
	figures.set_walking(false)
	for i in 240:
		figures._process(dt)
	check(absf(figures.group_factor("archer_0") - 1.0) < 0.02 or absf(figures.group_factor("infantry_0") - 1.0) < 0.02, "à l'arrêt, cadence 1")
	check(figures.bearer_tilt().is_equal_approx(Basis.IDENTITY), "à l'arrêt la hampe est droite")
	check(not figures.bearer_dynamic(), "hampe immobile à l'arrêt")
	marker.queue_free()
	await process_frame


func _clock(figures: ArmyFigures) -> float:
	var groups: Dictionary = figures._groups
	return float(groups[groups.keys()[0]]["clock"])


func _check_fleet() -> void:
	var pair := _make_marker("unit_knights", true)
	var marker: Node3D = pair[0]
	var figures: ArmyFigures = pair[1]
	check(figures.ship_count() > 0, "flotte construite")
	var low := Vector3.ZERO
	var heights: Array[float] = []
	for i in 120:
		figures._process(1.0 / 30.0)
		heights.append(figures.bearer_anchor().y)
		low = low.max(figures.bearer_tilt().get_euler().abs())
	var spread: float = float(heights.max()) - float(heights.min())
	check(spread > 0.01, "le pied de la hampe suit le pilonnement du navire (écart %.3f)" % spread)
	check(low.length() > 0.005, "la hampe suit le tangage/roulis (%.4f rad)" % low.length())
	marker.queue_free()
	await process_frame
