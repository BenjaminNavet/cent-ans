extends SceneTree

## Test du lot AS5 (restes statiques en shader) : vérifie par propriétés, sans rendu :
##  - données `fx/map_fire_wind.json` lues, sections présentes, éteintes par `enabled`/`--no-as5` ;
##  - `LifeEffects` : flammes MultiMesh sur les foyers d'incendie, matériau (planche FA2, couleurs,
##    cadence) posé, lumières vacillantes allumées de près et éteintes de loin ;
##  - bannières de maquette : amplitude et taille de toile posées sur le matériau ;
##  - imposteurs d'arbres de bataille : balancement posé sur le matériau.
## Usage : godot --headless --path game --script res://tests/as5_test.gd [-- --no-as5]

var _failures := 0


func _init() -> void:
	await process_frame
	_run()
	print("as5_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("as5_test: " + message)
	return condition


func _run() -> void:
	var off := CmdArgs.has("--no-as5")
	_check(MapFireWind.enabled() == not off, "enabled() follows --no-as5")
	var effects := LifeEffects.new()
	root.add_child(effects)
	effects.setup(null, null)
	if off:
		_check(effects.get("_flames") == null, "no flames with --no-as5")
		_check(MapFireWind.section("maquette_banner").is_empty(), "no banner wind with --no-as5")
		_check(MapFireWind.section("battle_tree_impostor").is_empty(), "no tree sway with --no-as5")
		return
	var cfg := MapFireWind.section("fire")
	_check(not cfg.is_empty(), "fire section loaded")
	var flames: MultiMeshInstance3D = effects.get("_flames")
	if not _check(flames != null, "flames instance created"):
		return
	var material: ShaderMaterial = effects.get("_flame_material")
	_check(material.get_shader_parameter("flipbook") is Texture2D, "flame flipbook bound")
	_check(is_equal_approx(float(material.get_shader_parameter("loops_per_s")), float(cfg["loops_per_s"])), "loops_per_s from data")
	_check(material.shader.code.contains("campaign_wind.gdshaderinc"), "flame shader shares the campaign wind")
	var points: Array = effects.get("_fire_points")
	for k in 6:
		points.append(effects.call("_fixed_point", Vector2(10.0 + k * 3.0, 20.0), 0.1, float(k) / 6.0))
	effects.call("_fill_flames")
	_check(flames.multimesh != null and flames.multimesh.instance_count == 6, "one flame per fire point")
	var size: Array = cfg["flame_size"]
	var buffer: PackedFloat32Array = (effects.get("_cpu_buffers") as Dictionary).get(flames, PackedFloat32Array())
	if _check(buffer.size() == 6 * 16, "flame buffer written"):
		var width := Vector3(buffer[0], buffer[4], buffer[8]).length()
		_check(width >= float(size[0]) * 0.79 and width <= float(size[0]) * 1.21, "flame width from data (%.3f)" % width)
	# Lumières : un cadre de caméra proche allume au plus `max_lights` foyers, loin elles s'éteignent.
	var light_cfg: Dictionary = cfg["light"]
	var lights: Array = effects.get("_flame_lights")
	_check(lights.size() == int(light_cfg["max_lights"]), "light pool size from data")
	var camera := Camera3D.new()
	root.add_child(camera)
	camera.global_position = Vector3(12.0, 5.0, 25.0)
	camera.make_current()
	effects.update_view(10.0, null)
	_check(flames.visible, "flames visible at the near tier")
	effects.set("_light_timer", 0.0)
	effects.update_view(10.0, null)
	_check(effects.flame_lights_on() == mini(6, int(light_cfg["max_lights"])), "lights on near the fires (%d)" % effects.flame_lights_on())
	effects.set("_light_timer", 0.0)
	effects.update_view(float(light_cfg["max_camera_distance"]) + 100.0, null)
	_check(effects.flame_lights_on() == 0, "lights off far away")
	# Bannières de maquette : matériau partagé.
	var layer := TownMaquetteLayer.new()
	var banner: ShaderMaterial = layer.call("_banner_material")
	var wind := MapFireWind.section("maquette_banner")
	_check(is_equal_approx(float(banner.get_shader_parameter("amplitude")), float(wind["amplitude"])), "banner amplitude from data")
	_check(banner.shader.code.contains("campaign_wind.gdshaderinc") and banner.shader.code.contains("TIME"), "banner shader waves with the wind")
	# Imposteurs d'arbres de bataille.
	var trees := BattleTrees.new()
	root.add_child(trees)
	trees.add_impostor_tile("as5", [Transform3D.IDENTITY], [Color.WHITE], [0])
	var sway := MapFireWind.section("battle_tree_impostor")
	var tree_material: ShaderMaterial = trees.impostor_material
	_check(tree_material != null and is_equal_approx(float(tree_material.get_shader_parameter("sway")), float(sway["sway"])), "tree sway from data")
	_check(tree_material.shader.code.contains("uniform float sway"), "impostor shader has sway")
	layer.free()
