extends SceneTree

## Test headless du lot DV (ADR 0124) : deux vues de la carte de campagne (vue normale 3D, vue
## stratégique parchemin), un seul fondu autour de `ZoomTiers.strategic_threshold`.
## Usage : godot --headless --path game --script res://tests/dv_two_views_test.gd

const ENABLED := false  # DV3 : activé une fois la vague 1 intégrée

var _failures := 0


func _init() -> void:
	await process_frame
	if not ENABLED:
		print("dv_two_views_test: SKIPPED")
		quit(0)
		return
	var tiers := ZoomTiers.load_default()
	_check(tiers.strategic_weight(1000.0) < 0.001, "normal view at 1000")
	_check(tiers.strategic_weight(1400.0) > 0.999, "strategic view at 1400")
	_check(tiers.tier_at(100.0) == ZoomTiers.Tier.NEAR, "normal view tier")
	_check(tiers.tier_at(3.0) == ZoomTiers.Tier.VALLEY, "valley tier")
	_check(tiers.tier_at(0.5) == ZoomTiers.Tier.SITE, "site tier")
	_check(tiers.tier_at(1400.0) == ZoomTiers.Tier.STRATEGIC, "strategic tier")
	_check(tiers.model_range > 1100.0, "models reach the normal view edge")
	# DV3 : aucun quad de pictogramme ; maquettes visibles à 1100 ; parchemin avec vignettes.
	if _failures == 0:
		print("dv_two_views_test: OK")
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> void:
	if not condition:
		_failures += 1
		push_error("dv_two_views_test FAILED: " + message)
