class_name BattleBannerLayer
extends Node3D

## Drapeaux-repères des régiments : un mât et une étoffe par régiment, tournés de trois
## quarts vers la caméra, qui flottent dans le vent de près (BV3) et se froissent en déroute.
## Extrait de `BattleScene` : la scène compose cette couche et lui passe les régiments à chaque image.

const BANNER_HEIGHT := 7.0
const BANNER_SHADER := preload("res://shaders/battle_banner.gdshader")

var _scene: BattleScene = null
var _banners: Dictionary = {}  # id -> {node, flag_mat, routing}


func attach(scene: BattleScene) -> void:
	_scene = scene
	name = "Banners"
	scene.add_child(self)


## Un drapeau par régiment (appelé une fois, à la construction de la scène).
func build(units: Array) -> void:
	for unit in units:
		_make(unit)


## Matériaux d'étoffe par régiment : `BattleStandards.apply_wind` y pose le vent.
func flag_materials() -> Array:
	var materials: Array = []
	for id in _banners:
		materials.append((_banners[id] as Dictionary)["flag_mat"])
	return materials


func _make(unit: Dictionary) -> void:
	var id := int(unit["id"])
	var side := str(unit["side"])
	var node := Node3D.new()
	node.name = "Banner%d" % id
	add_child(node)
	var pole := MeshInstance3D.new()
	pole.mesh = BattleMeshes.pole()
	pole.scale = Vector3(1, BANNER_HEIGHT, 1)
	pole.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	node.add_child(pole)
	var flag := MeshInstance3D.new()
	var flag_mat := ShaderMaterial.new()
	flag_mat.shader = BANNER_SHADER
	flag_mat.set_shader_parameter("livery", _scene.side_colors[side])
	flag_mat.set_shader_parameter("phase", float(id) * 1.7)
	var cloth := cloth_for(unit, str((_scene.setup[side] as Dictionary).get("faction", "")))
	var flag_size: Vector2 = cloth["size"]
	flag.mesh = BattleMeshes.flag(flag_size.x, flag_size.y)
	flag_mat.set_shader_parameter("heraldry", cloth["texture"])
	flag_mat.set_shader_parameter("has_heraldry", cloth["texture"] != null)
	flag_mat.set_shader_parameter("full_texture", cloth["full"])
	flag_mat.set_shader_parameter("flag_length", flag_size.x)
	flag.material_override = flag_mat
	flag.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	flag.position = Vector3(0.03, BANNER_HEIGHT - 0.05, 0)
	node.add_child(flag)
	_banners[id] = {"node": node, "flag_mat": flag_mat, "routing": false}


## Étoffe d'un drapeau de régiment : bannière peinte de la faction (`heraldry/banners/`,
## 256×512, tissu dans le haut, bas transparent) pour la noblesse, fanion à queue d'aronde (4:1)
## pour les autres, étendards royaux pour le général de France / d'Angleterre ; à défaut, centre
## de l'écu de la faction (repli). DA1b : le général et les unités nobles de sa retenue portent
## la bannière de sa maison (`heraldry/banners/houses/`) quand elle existe.
func cloth_for(unit: Dictionary, faction: String) -> Dictionary:
	var dir := "res://assets/heraldry/banners/"
	var noble := str(unit.get("type", "")) in ["unit_knights", "unit_men_at_arms_foot"]
	var candidates: Array = []
	if bool(unit.get("is_general", false)) and faction in ["fac_france", "fac_england"]:
		# Pas de quartier : oriflamme (France) / dragon (Angleterre) ; sinon Saint-Georges pour
		# l'armée royale anglaise, bannière de la faction pour la française.
		var side := str(unit.get("side", ""))
		var no_quarter: bool = _scene.battle != null and bool(_scene.battle.call("get_no_quarter", side))
		if no_quarter:
			candidates.append([dir + ("oriflamme.png" if faction == "fac_france" else "dragon.png"), Vector2(1.3, 2.6)])
		elif faction == "fac_england":
			candidates.append([dir + "st_george.png", Vector2(1.3, 2.6)])
	var house := HouseArms.id_of(str(_scene._side_houses.get(str(unit.get("side", "")), "")))
	if house != "" and (bool(unit.get("is_general", false)) or BattleStandards.is_house_retinue(unit)):
		var house_banner: Array = [dir + "houses/%s_banner.png" % house, Vector2(1.3, 2.6)]
		var no_quarter_first := not candidates.is_empty() and (str(candidates[0][0]).ends_with("oriflamme.png") or str(candidates[0][0]).ends_with("dragon.png"))
		candidates.insert(1 if no_quarter_first else 0, house_banner)
	if noble or str(unit.get("render", "")) == "siege":
		candidates.append([dir + "%s_banner.png" % faction, Vector2(1.3, 2.6)])
	else:
		candidates.append([dir + "%s_pennon.png" % faction, Vector2(3.0, 0.75)])
		candidates.append([dir + "%s_banner.png" % faction, Vector2(1.3, 2.6)])
	for candidate in candidates:
		var texture := PortraitLoader.load_texture(candidate[0])
		if texture != null:
			return {"texture": texture, "size": candidate[1], "full": true}
	return {"texture": PortraitLoader.heraldry_texture(faction), "size": Vector2(2.6, 1.7), "full": false}


## Pose les drapeaux d'après les régiments de l'image. `standards` (peut être nul) donne le vent
## et masque le drapeau-repère quand l'étendard porté est affiché. Pose, échelle et lacet ne sont
## réécrits que s'ils ont changé (bataille en pause, caméra fixe : aucune écriture sur le nœud).
func update(units: Array, banner_scale: float, cam_yaw: float, standards: BattleStandards) -> void:
	var scale_vector := Vector3.ONE * banner_scale
	var wind_mix := smoothstep(1.0, 2.5, banner_scale)
	var hide_markers := standards != null and banner_scale <= standards.hide_scale()
	var downwind_yaw := standards.downwind_yaw() if standards != null else 0.0
	for unit in units:
		var id := int(unit["id"])
		var banner: Dictionary = _banners[id]
		var node: Node3D = banner["node"]
		var present: bool = unit["present"]
		if node.visible != present:
			node.visible = present
		if not present:
			continue
		var unit_position := Vector3(float(unit["x"]), float(unit["y"]), float(unit["z"]))
		if node.position != unit_position:
			node.position = unit_position
		if node.scale != scale_vector:
			node.scale = scale_vector
		var yaw := cam_yaw
		var flag_visible := true
		if standards != null:
			# De près, le drapeau-repère flotte dans le vent, et s'efface devant l'étendard
			# porté quand celui-ci est affiché.
			yaw = lerp_angle(downwind_yaw, cam_yaw, wind_mix)
			flag_visible = not (hide_markers and standards.handles(id))
		if node.rotation.y != yaw:
			node.rotation.y = yaw
		if node.visible != flag_visible:
			node.visible = flag_visible
		var routing := str(unit["state"]) == "routing"
		if routing != bool(banner["routing"]):
			banner["routing"] = routing
			(banner["flag_mat"] as ShaderMaterial).set_shader_parameter("routing", routing)
