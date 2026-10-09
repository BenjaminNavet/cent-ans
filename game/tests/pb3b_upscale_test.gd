extends TestCase

## PB3b (ADR 0080) : mise à l'échelle 3D MetalFX de `RenderQuality` (préréglages, réglage du
## joueur, repli FSR hors Metal, anticrénelage coupé par le temporel).
## Usage : godot --headless --path game --script res://tests/pb3b_upscale_test.gd
## Code de sortie 0 si tout passe, 1 sinon.


func _init() -> void:
	await process_frame
	for level: String in RenderQuality.LEVELS:
		var p: Dictionary = RenderQuality.presets()[level]
		check(p.has("upscale_mode") and p.has("upscale_scale"), "%s: upscale keys" % level)
		var mode := str(p["upscale_mode"])
		check(mode in [RenderQuality.UPSCALE_OFF, RenderQuality.UPSCALE_SPATIAL, RenderQuality.UPSCALE_TEMPORAL], "%s: known upscale mode %s" % [level, mode])
		var scale := float(p["upscale_scale"])
		check(scale >= 0.5 and scale <= 1.0, "%s: upscale scale in [0.5, 1] (%.2f)" % [level, scale])
	# Repli hors Metal : FSR 1 pour le spatial, FSR 2 pour le temporel.
	check(RenderQuality.scaling_mode_for(RenderQuality.UPSCALE_SPATIAL, true) == Viewport.SCALING_3D_MODE_METALFX_SPATIAL, "spatial on Metal")
	check(RenderQuality.scaling_mode_for(RenderQuality.UPSCALE_TEMPORAL, true) == Viewport.SCALING_3D_MODE_METALFX_TEMPORAL, "temporal on Metal")
	check(RenderQuality.scaling_mode_for(RenderQuality.UPSCALE_SPATIAL, false) == Viewport.SCALING_3D_MODE_FSR, "spatial fallback FSR")
	check(RenderQuality.scaling_mode_for(RenderQuality.UPSCALE_TEMPORAL, false) == Viewport.SCALING_3D_MODE_FSR2, "temporal fallback FSR2")
	check(RenderQuality.scaling_mode_for(RenderQuality.UPSCALE_OFF, true) == Viewport.SCALING_3D_MODE_BILINEAR, "off is bilinear")
	# ADR 0123 : budget de pixels de la référence 1080p pour « Automatique ».
	var spatial := RenderQuality.UPSCALE_SPATIAL
	check(is_equal_approx(RenderQuality.budget_scale(spatial, 0.75, 1920.0 * 1080.0), 0.75), "budget: 1080p keeps preset scale")
	check(is_equal_approx(RenderQuality.budget_scale(spatial, 0.75, 1440.0 * 900.0), 0.75), "budget: smaller window never upscales")
	var retina := RenderQuality.budget_scale(spatial, 0.75, 2624.0 * 1644.0)
	check(retina > 0.51 and retina < 0.53, "budget: Retina 2624x1644 at 0.52 (%.3f)" % retina)
	check(is_equal_approx(RenderQuality.budget_scale(spatial, 0.75, 3840.0 * 2160.0), RenderQuality.min_upscale_scale()), "budget: 4K floored")
	check(RenderQuality.budget_scale(RenderQuality.UPSCALE_OFF, 1.0, 3840.0 * 2160.0) == 1.0, "budget: off stays native")
	# Configurations des bancs.
	var parsed := RenderQuality.parse_upscale("metalfx_t:0.67")
	check(parsed["mode"] == RenderQuality.UPSCALE_TEMPORAL and is_equal_approx(float(parsed["scale"]), 0.67), "parse metalfx_t:0.67")
	parsed = RenderQuality.parse_upscale("off")
	check(parsed["mode"] == RenderQuality.UPSCALE_OFF and float(parsed["scale"]) == 1.0, "parse off")
	# Réglage du joueur : « auto » par défaut, choix explicite prioritaire sur le préréglage.
	var settings: Node = root.get_node_or_null("Settings")
	check(settings != null, "Settings autoload")
	if settings != null:
		var path := "user://pb3b_upscale_%d.cfg" % OS.get_process_id()
		settings.call("use_test_file", path)
		check(str(settings.call("get_value", "video/upscale")) == "auto", "default is auto")
		check(RenderQuality.upscale() == RenderQuality.preset_upscale(RenderQuality.preset()), "auto follows the preset")
		settings.call("set_value", "video/upscale", "performance")
		var up := RenderQuality.upscale()
		check(float(up["scale"]) < 1.0 and up["mode"] != RenderQuality.UPSCALE_OFF, "performance upscales")
		settings.call("set_value", "video/upscale", "off")
		check(RenderQuality.upscale()["mode"] == RenderQuality.UPSCALE_OFF, "off honoured")
		settings.call("set_value", "video/upscale", "auto")
		# Réglages > Affichage : liste « Mise à l'échelle » (Automatique + trois choix).
		var menu := SettingsMenu.new()
		root.add_child(menu)
		await process_frame
		var controls: Dictionary = menu.get("_controls")
		var option := controls.get("video/upscale") as OptionButton
		check(option != null and option.item_count == RenderQuality.UPSCALE_CHOICES.size(), "settings menu: upscale option")
		if option != null:
			check(option.get_item_text(0).begins_with("Automatique ("), "settings menu: auto label %s" % option.get_item_text(0))
			option.item_selected.emit(2)  # MetalFX qualité
			check(str(settings.call("get_value", "video/upscale")) == "quality", "settings menu: choice saved")
		menu.queue_free()
		settings.call("set_value", "video/upscale", "auto")
		DirAccess.remove_absolute(path)
	# Le temporel coupe MSAA et FXAA ; le spatial garde l'anticrénelage du préréglage.
	var viewport := SubViewport.new()
	root.add_child(viewport)
	var high: Dictionary = RenderQuality.presets()["high"]
	RenderQuality.upscale_override = "metalfx_t:0.75"
	RenderQuality.apply_upscale(viewport, high)
	check(viewport.msaa_3d == Viewport.MSAA_DISABLED and viewport.screen_space_aa == Viewport.SCREEN_SPACE_AA_DISABLED, "temporal disables MSAA and FXAA")
	check(is_equal_approx(viewport.scaling_3d_scale, 0.75), "temporal scale applied")
	RenderQuality.upscale_override = "metalfx_s:0.75"
	RenderQuality.apply_upscale(viewport, high)
	check(viewport.msaa_3d == high["msaa"], "spatial keeps preset MSAA")
	check(RenderQuality.is_temporal(viewport.scaling_3d_mode) == false, "spatial is not temporal")
	RenderQuality.upscale_override = "off"
	RenderQuality.apply_upscale(viewport, high)
	check(viewport.scaling_3d_mode == Viewport.SCALING_3D_MODE_BILINEAR and viewport.scaling_3d_scale == 1.0, "off restores native resolution")
	RenderQuality.upscale_override = ""
	viewport.queue_free()
	finish()


