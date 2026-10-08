class_name SeaLife
extends RefCounted

## Lot ME1 (chantier DN, mer vivante) : fonds clairs, crêtes du large, saturation bornée, liseré de
## ressac lisible, estrans et sillages des flottes. Réglages dans `data/fx/sea_life.json` (schéma
## `fx_sea_life.schema.json`), jamais codés en dur ; shader `sea_life.gdshaderinc`. Rendu seulement.
## `--no-sea-life` ou fichier absent : le shader garde `sea_life_on` = false (rendu d'avant).

const SPEC_FILE := "fx/sea_life.json"

static var _lookup := JsonLookup.new(SPEC_FILE, {}, "", "shallows")
static var _tide_texture: ImageTexture = null
static var _tide_size: Vector2i = Vector2i.ZERO


static func enabled() -> bool:
	return not CmdArgs.has("--no-sea-life") and not spec().is_empty()


static func spec() -> Dictionary:
	return _lookup.data()


static func reload() -> void:
	_lookup.reload()
	_tide_texture = null


## Zone d'estran contenant `px` (pixels carte) : {id, width_px}, {} hors zone. La dernière gagne.
static func tide_zone_at(px: Vector2) -> Dictionary:
	var found := {}
	for zone: Dictionary in (spec().get("tide", {}) as Dictionary).get("zones", []):
		if Geometry2D.is_point_in_polygon(px, CoastLook.polygon_of(zone)):
			found = {"id": str(zone["id"]), "width_px": float(zone["width_px"])}
	return found


## Texture des estrans : R = largeur de vase maximale de la zone / `tide.max_px`.
static func tide_texture(map_size: Vector2i) -> ImageTexture:
	if _tide_texture != null and _tide_size == map_size:
		return _tide_texture
	var cell := maxi(int(spec().get("cell_px", 16)), 1)
	var image := Image.create(ceili(map_size.x / float(cell)), ceili(map_size.y / float(cell)), false, Image.FORMAT_R8)
	var tide: Dictionary = spec().get("tide", {})
	var max_px := maxf(float(tide.get("max_px", 8.0)), 0.001)
	for zone: Dictionary in tide.get("zones", []):
		CoastLook.fill_polygon(image, CoastLook.polygon_of(zone), cell, Color(clampf(float(zone["width_px"]) / max_px, 0.0, 1.0), 0.0, 0.0))
	_tide_texture = ImageTexture.create_from_image(image)
	_tide_size = map_size
	return _tide_texture


## Pose les réglages sur le matériau de la mer ; renvoie true si la mer vivante est active.
static func apply(material: ShaderMaterial, map_size: Vector2i) -> bool:
	if material == null:
		return false
	material.set_shader_parameter("sea_life_on", false)
	if not enabled():
		return false
	var data := spec()
	material.set_shader_parameter("sea_life_on", true)
	material.set_shader_parameter("sea_sat_max", float((data.get("color", {}) as Dictionary).get("saturation_max", 1.0)))
	material.set_shader_parameter("sea_gain", float((data.get("color", {}) as Dictionary).get("brightness", 1.0)))
	material.set_shader_parameter("sea_cloud_shadow_gain", float((data.get("color", {}) as Dictionary).get("cloud_shadow_gain", 1.0)))
	material.set_shader_parameter("surf_min_screen_px", float((data.get("surf", {}) as Dictionary).get("min_screen_px", 0.0)))
	var shallows: Dictionary = data.get("shallows", {})
	material.set_shader_parameter("shallows_color", _vec3(shallows.get("color"), Vector3(0.2, 0.4, 0.38)))
	material.set_shader_parameter("shallows_depth_m", float(shallows.get("depth_m", 40.0)))
	material.set_shader_parameter("shallows_amount", float(shallows.get("amount", 0.0)))
	material.set_shader_parameter("shallows_caustic", float(shallows.get("caustic", 0.0)))
	material.set_shader_parameter("shallows_caustic_scale", float(shallows.get("caustic_scale", 0.9)))
	var crests: Dictionary = data.get("crests", {})
	material.set_shader_parameter("crest_color", _vec3(crests.get("color"), Vector3(0.46, 0.55, 0.58)))
	material.set_shader_parameter("crest_amount", float(crests.get("amount", 0.0)))
	material.set_shader_parameter("crest_depth_m", _vec2(crests.get("depth_m"), Vector2(40.0, 140.0)))
	material.set_shader_parameter("crest_frequency", float(crests.get("frequency", 2.4)))
	material.set_shader_parameter("crest_speed", float(crests.get("speed", 0.55)))
	material.set_shader_parameter("crest_dir", _vec2(crests.get("direction"), Vector2(0.8, 0.6)))
	var tide: Dictionary = data.get("tide", {})
	material.set_shader_parameter("tide_zones", tide_texture(map_size))
	material.set_shader_parameter("tide_period_s", float(tide.get("period_s", 150.0)))
	material.set_shader_parameter("tide_max_px", float(tide.get("max_px", 8.0)))
	material.set_shader_parameter("tide_high_cover", float(tide.get("high_cover", 0.9)))
	material.set_shader_parameter("tide_mud_color", _vec3(tide.get("mud_color"), Vector3(0.13, 0.115, 0.085)))
	material.set_shader_parameter("tide_wet_color", _vec3(tide.get("wet_color"), Vector3(0.22, 0.22, 0.19)))
	material.set_shader_parameter("tide_mud_alpha", float(tide.get("mud_alpha", 0.96)))
	return true


static func _vec3(values: Variant, fallback: Vector3) -> Vector3:
	if values is Array and (values as Array).size() >= 3:
		return Vector3(float(values[0]), float(values[1]), float(values[2]))
	return fallback


static func _vec2(values: Variant, fallback: Vector2) -> Vector2:
	if values is Array and (values as Array).size() >= 2:
		return Vector2(float(values[0]), float(values[1]))
	return fallback


# --- Sillage des flottes -----------------------------------------------------------------

const WAKE_SHADER := preload("res://shaders/fleet_wake.gdshader")
static var _wake_material: ShaderMaterial = null


## Sillage d'un navire (ruban triangulaire derrière la poupe, +X = avant) ; `ship_scale` règle la
## taille ; null si désactivé. `set_wake_strength` l'anime (en route / au mouillage).
static func make_wake(ship_scale: float) -> MeshInstance3D:
	var wake: Dictionary = spec().get("wake", {})
	if not enabled() or not bool(wake.get("enabled", false)):
		return null
	var length := float(wake.get("length", 34.0)) * ship_scale
	var w0 := float(wake.get("width_start", 4.0)) * ship_scale
	var w1 := float(wake.get("width_end", 15.0)) * ship_scale
	var stern := float(wake.get("stern_x", -6.5)) * ship_scale
	var vertices := PackedVector3Array()
	var uvs := PackedVector2Array()
	var indices := PackedInt32Array()
	var steps := 8
	for i in steps + 1:
		var t := float(i) / steps
		var half := lerpf(w0, w1, t) * 0.5
		var x := stern - t * length
		vertices.append(Vector3(x, 0.0, -half))
		vertices.append(Vector3(x, 0.0, half))
		uvs.append(Vector2(t, 0.0))
		uvs.append(Vector2(t, 1.0))
		if i < steps:
			var a := i * 2
			indices.append_array([a, a + 1, a + 2, a + 1, a + 3, a + 2])
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_TEX_UV] = uvs
	arrays[Mesh.ARRAY_INDEX] = indices
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	if _wake_material == null:
		_wake_material = ShaderMaterial.new()
		_wake_material.shader = WAKE_SHADER
		_wake_material.set_shader_parameter("wake_color", _vec3(wake.get("color"), Vector3(0.8, 0.86, 0.86)))
		_wake_material.set_shader_parameter("wake_speed", float(wake.get("speed", 0.9)))
	var instance := MeshInstance3D.new()
	instance.name = "Wake"
	instance.mesh = mesh
	instance.material_override = _wake_material
	instance.position.y = float(wake.get("y", 0.12))
	instance.layers = 2
	instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	set_wake_strength(instance, false)
	return instance


static func set_wake_strength(wake: MeshInstance3D, moving: bool) -> void:
	if wake == null:
		return
	var data: Dictionary = spec().get("wake", {})
	wake.set_instance_shader_parameter("wake_alpha", float(data.get("alpha_moving" if moving else "alpha_idle", 0.5)))
