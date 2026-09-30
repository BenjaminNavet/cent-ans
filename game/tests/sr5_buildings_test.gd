extends SceneTree

## Lot SR5 : usure des bâtiments. Vérifie que les shaders `building_atlas`, `town_building` et
## `building_pbr` se compilent avec l'uniforme `aging`, que les matières texturées du kit sont des
## ShaderMaterial partagés avec usure (StandardMaterial3D avec `--no-sr5`), que l'atlas porte
## l'usure (0 avec `--no-sr5`) et que le nombre de matériaux distincts d'un jeu de modèles du kit
## est le même dans les deux modes (pas d'appel de dessin en plus).
## Usage : godot --headless --path game --script res://tests/sr5_buildings_test.gd [-- --no-sr5]

const SHADERS := [
	"res://shaders/building_atlas.gdshader",
	"res://shaders/town_building.gdshader",
	"res://shaders/building_pbr.gdshader",
]

var ok := true


func _fail(message: String) -> void:
	print("SR5 : " + message)
	ok = false


func _has_uniform(shader: Shader, uniform_name: String) -> bool:
	for u in shader.get_shader_uniform_list():
		if str(u["name"]) == uniform_name:
			return true
	return false


## Matériaux distincts (et nombre de surfaces) des modèles du kit après remplacement.
func _kit_materials(variant: String) -> Vector2i:
	var unique := {}
	var surfaces := 0
	for model_name in BuildingKit.manifest():
		var mesh := BuildingKit.mesh(str(model_name), variant)
		if mesh == null:
			continue
		for i in mesh.get_surface_count():
			surfaces += 1
			var mat := mesh.surface_get_material(i)
			if mat != null:
				unique[mat.get_instance_id()] = true
	return Vector2i(unique.size(), surfaces)


## Toutes les matières nommées (texturées et unies) dans la variante donnée.
func _named_materials(variant: String) -> Array:
	var out := []
	for name in BuildingMaterials.SPECS.keys() + BuildingMaterials.PLAIN.keys():
		out.append(BuildingMaterials.material(str(name), variant))
	return out


func _check_mode(enabled: bool) -> Dictionary:
	BuildingMaterials.sr5_enabled = enabled
	BuildingMaterials.clear_cache()
	BuildingMaterials.material("Plaster")
	var counts := {}
	for variant in ["", "snow", "far"]:
		for name in BuildingMaterials.SPECS.keys():
			var mat := BuildingMaterials.material(str(name), variant)
			if enabled:
				var shader_mat := mat as ShaderMaterial
				if shader_mat == null or shader_mat.shader != BuildingMaterials.PBR_SHADER:
					_fail("%s/%s n'est pas un ShaderMaterial building_pbr" % [name, variant])
					continue
				if float(shader_mat.get_shader_parameter("aging")) <= 0.0:
					_fail("%s/%s sans usure" % [name, variant])
			elif not (mat is StandardMaterial3D):
				_fail("%s/%s devrait être un StandardMaterial3D avec --no-sr5" % [name, variant])
			if mat != BuildingMaterials.material(str(name), variant):
				_fail("%s/%s n'est pas partagé" % [name, variant])
		var atlas := BuildingMaterials.material("Building", variant) as ShaderMaterial
		var aging := float(atlas.get_shader_parameter("aging"))
		if enabled and aging <= 0.0:
			_fail("atlas %s sans usure" % variant)
		if not enabled and aging != 0.0:
			_fail("atlas %s : usure %.2f avec --no-sr5" % [variant, aging])
		var named := {}
		for mat in _named_materials(variant):
			named[mat.get_instance_id()] = true
		counts[variant] = [named.size(), _kit_materials(variant)]
	print("SR5 %s : %s" % ["actif" if enabled else "--no-sr5", counts])
	return counts


func _init() -> void:
	for path in SHADERS:
		var shader := load(path) as Shader
		if shader == null or shader.get_shader_uniform_list().is_empty():
			_fail("%s ne compile pas" % path)
		elif not _has_uniform(shader, "aging"):
			_fail("%s sans uniforme aging" % path)
	var flag := OS.get_cmdline_user_args().has("--no-sr5")
	if BuildingMaterials.sr5_enabled == flag:
		_fail("drapeau --no-sr5 mal lu (sr5_enabled = %s)" % BuildingMaterials.sr5_enabled)
	var initial := BuildingMaterials.sr5_enabled
	var with_sr5 := _check_mode(true)
	var without := _check_mode(false)
	if str(with_sr5) != str(without):
		_fail("nombre de matériaux différent : %s contre %s" % [with_sr5, without])
	if int(with_sr5[""][0]) == 0:
		_fail("aucune matière chargée")
	BuildingMaterials.sr5_enabled = initial
	BuildingMaterials.clear_cache()
	print("SR5 OK" if ok else "SR5 ECHEC")
	quit(0 if ok else 1)
