extends SceneTree

## Test headless du lot DV (ADR 0124) : deux vues de la carte de campagne (vue normale 3D, vue
## stratégique parchemin), un seul fondu autour de `ZoomTiers.strategic_threshold`.
## Usage : godot --headless --path game --script res://tests/dv_two_views_test.gd

const ENABLED := true

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
	# Plus aucun pictogramme de ville : catalogue et shader ne portent que l'écu.
	_check(not SettlementMarkers.new().has_method("pictogram_for"), "no pictogram catalogue")
	var icon_shader := load("res://shaders/settlement_icon.gdshader") as Shader
	_check(icon_shader != null and not icon_shader.code.contains("uniform sampler2D atlas"), "shield-only settlement shader")
	_check(not ResourceLoader.exists("res://assets/map/markers/settlement_markers.png"), "pictogram atlas removed")
	# Le parchemin dessine ses vignettes de villes (seule représentation en vue stratégique).
	var overlay := ParchmentOverlay.new()
	_check(overlay.draw_towns, "parchment draws its town vignettes")
	overlay.free()
	# Maquettes visibles à 1100 : `settlements_render_test.gd` § 6 (nécessite la carte chargée).
	if _failures == 0:
		print("dv_two_views_test: OK")
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> void:
	if not condition:
		_failures += 1
		push_error("dv_two_views_test FAILED: " + message)
