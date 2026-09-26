extends SceneTree

## Test headless du lot SZ4 (objets à l'échelle aux paliers intermédiaires) :
##  1. `MapPropScale` : 1 au loin (lisibilité stratégique conservée), taille réelle de près,
##     décroissance monotone et continue (aucun saut entre deux distances voisines) ;
##  2. réglages lus depuis `res://resources/map_prop_scale.tres`.
## Usage : godot --headless --path game --script res://tests/sz4_prop_scale_test.gd

var _failures := 0


func _init() -> void:
	await process_frame
	_test_curve()
	if _failures > 0:
		push_error("sz4_prop_scale_test: %d failure(s)" % _failures)
		quit(1)
		return
	print("sz4_prop_scale_test: OK")
	quit(0)


func _check(condition: bool, label: String) -> void:
	if not condition:
		_failures += 1
		push_error("FAIL: " + label)


func _test_curve() -> void:
	var props := MapPropScale.shared()
	_check(props != null and ResourceLoader.exists("res://resources/map_prop_scale.tres"), "shared resource")
	for d in [60.0, 150.0, 620.0, props.shrink_start]:
		_check(is_equal_approx(props.tree_scale(d), 1.0) and is_equal_approx(props.hamlet_scale(d), 1.0), "far scale is 1 at d=%.1f" % d)
	_check(is_equal_approx(props.windmill_scale(props.shrink_end), props.windmill_ratio), "real size at shrink_end")
	_check(is_equal_approx(props.hamlet_scale(1.0), props.hamlet_ratio), "real size below shrink_end")
	var previous := 1.0
	var max_jump := 0.0
	var d := props.shrink_start + 1.0
	while d > props.shrink_end - 1.0:
		var s := props.windmill_scale(d)
		_check(s <= previous + 1e-6, "monotonic at d=%.2f" % d)
		max_jump = maxf(max_jump, absf(log(previous) - log(s)))
		previous = s
		d -= 0.05
	# Pas de saut : sur un pas de 0,05 unité, l'échelle varie de moins de 6 %.
	_check(max_jump < 0.06, "continuous (max log step %.3f)" % max_jump)
	_check(not props.needs_rewrite(0.5, 0.51) and props.needs_rewrite(0.5, 0.45) and props.needs_rewrite(0.9, 1.0), "rewrite step")
