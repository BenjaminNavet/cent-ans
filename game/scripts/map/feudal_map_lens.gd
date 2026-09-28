class_name FeudalMapLens
extends RefCounted

## Lot FE6 (spec FE § 6) : filtre de carte « Féodalité » (MF1). Fond : couleur du souverain au
## sommet de la chaîne de la province ; hachures : couleur du tenant direct de son titre quand ce
## n'est pas le souverain ; écu parti (tenant | seigneur de la province) sur les provinces en
## double allégeance (la Guyenne anglaise tenue du roi de France). Les valeurs viennent de
## `CampaignSim.get_feudal_map` : ce fichier ne fait que les traduire en couleurs et en écus.

const SHIELD_SIZE := Vector2i(48, 56)
const NEUTRAL := Color(0.62, 0.60, 0.56)

## Dernière lecture, par id de province : `{sovereign, holder, second_lord}`.
var cells: Dictionary = {}
var _shields: Node3D = null
var _shield_cache: Dictionary = {}


static func available(sim: Object) -> bool:
	return sim != null and sim.has_method("get_feudal_map")


static func faction_color(faction_id: String) -> Color:
	if faction_id == "":
		return Color(0, 0, 0, 0)
	var facade := Engine.get_main_loop().root.get_node_or_null("SimFacade") if Engine.get_main_loop() is SceneTree else null
	var color: Color = facade.call("faction_color", faction_id) if facade != null else NEUTRAL
	color.a = 1.0
	return color


## Lit le cœur : `{colors, hatch, doubles}` — couleurs de fond et de hachure (une par id, dans
## l'ordre de `ids`) et `doubles` : `[[province, tenant, seigneur]]` des doubles allégeances.
func read(sim: Object, ids: PackedStringArray) -> Dictionary:
	var rows: Array = sim.call("get_feudal_map", ids)
	cells.clear()
	var colors := PackedColorArray()
	var hatch := PackedColorArray()
	var doubles: Array = []
	for index in ids.size():
		var row: Dictionary = rows[index] if index < rows.size() else {}
		if row.is_empty():
			colors.append(Color(0, 0, 0, 0))
			hatch.append(Color(0, 0, 0, 0))
			continue
		cells[ids[index]] = row
		var sovereign := str(row.get("sovereign", ""))
		var holder := str(row.get("holder", ""))
		colors.append(faction_color(sovereign))
		hatch.append(faction_color(holder) if holder != "" and holder != sovereign else Color(0, 0, 0, 0))
		var second := str(row.get("second_lord", ""))
		if second != "":
			doubles.append([ids[index], holder, second])
	return {"colors": colors, "hatch": hatch, "doubles": doubles}


## Texte de l'infobulle de survol : « tenu par Bourgogne, sous France ; double allégeance ».
func hover_text(province_id: String) -> String:
	var row: Dictionary = cells.get(province_id, {})
	if row.is_empty():
		return ""
	var facade := Engine.get_main_loop().root.get_node_or_null("SimFacade") if Engine.get_main_loop() is SceneTree else null
	var name_of := func(id: String) -> String:
		return str(facade.call("faction_short_name", id)) if facade != null else id
	var sovereign := str(row.get("sovereign", ""))
	var holder := str(row.get("holder", ""))
	var text := "royaume de %s" % name_of.call(sovereign) if holder == sovereign or holder == "" \
		else "tenu par %s, sous %s" % [name_of.call(holder), name_of.call(sovereign)]
	var second := str(row.get("second_lord", ""))
	if second != "":
		text += " ; double allégeance (fief de %s)" % name_of.call(second)
	return text


## Entrées de légende (catégories du filtre).
static func legend_entries() -> Array:
	return [[Color(0.55, 0.42, 0.25), "Fond : le royaume (souverain)"],
		[Color(0.35, 0.45, 0.60), "Hachures : le tenant direct (grand vassal)"],
		[Color(0.75, 0.62, 0.20), "Écu parti : double allégeance"]]


## Écus partis sur la carte 3D (billboards au centre des provinces).
func place_shields(map: Node, doubles: Array) -> void:
	clear_shields()
	if not (map is Node3D) or map.get("map_data") == null:
		return
	_shields = Node3D.new()
	_shields.name = "FeudalShields"
	map.add_child(_shields)
	for entry in doubles:
		var province := str(entry[0])
		var centroid: Vector2 = map.map_data.centroid_of_id(province)
		var sprite := Sprite3D.new()
		sprite.name = "Shield_%s" % province
		sprite.texture = party_shield(str(entry[1]), str(entry[2]))
		sprite.billboard = BaseMaterial3D.BILLBOARD_ENABLED
		sprite.no_depth_test = true
		sprite.fixed_size = true
		sprite.pixel_size = 0.0006
		sprite.render_priority = 10
		sprite.position = Vector3(centroid.x, map.map_data.surface_world_at(centroid.x, centroid.y) + 0.6, centroid.y)
		_shields.add_child(sprite)


func shield_count() -> int:
	return _shields.get_child_count() if _shields != null and is_instance_valid(_shields) else 0


func clear_shields() -> void:
	if _shields != null and is_instance_valid(_shields):
		_shields.queue_free()
	_shields = null


## Écu parti : moitié gauche aux armes de `left`, moitié droite à celles de `right` (à défaut de
## blason, leur couleur), bordé d'un filet sombre.
func party_shield(left: String, right: String) -> Texture2D:
	var key := "%s|%s" % [left, right]
	if _shield_cache.has(key):
		return _shield_cache[key]
	var image := Image.create(SHIELD_SIZE.x, SHIELD_SIZE.y, false, Image.FORMAT_RGBA8)
	image.fill(Color(0, 0, 0, 0))
	var halves := [_arms_image(left), _arms_image(right)]
	var mid := SHIELD_SIZE.x / 2
	for y in SHIELD_SIZE.y:
		for x in SHIELD_SIZE.x:
			if not _inside_shield(x, y):
				continue
			var source: Variant = halves[0] if x < mid else halves[1]
			var color: Color = (source as Image).get_pixel(x, y) if source is Image else faction_color(left if x < mid else right)
			color.a = 1.0
			if _on_edge(x, y) or x == mid:
				color = Color(0.12, 0.08, 0.05)
			image.set_pixel(x, y, color)
	var texture := ImageTexture.create_from_image(image)
	_shield_cache[key] = texture
	return texture


func _arms_image(faction_id: String) -> Variant:
	var texture := PortraitLoader.heraldry_texture(faction_id)
	if texture == null:
		return null
	var image := texture.get_image()
	if image == null:
		return null
	image = image.duplicate()
	if image.is_compressed():
		image.decompress()
	image.convert(Image.FORMAT_RGBA8)
	image.resize(SHIELD_SIZE.x, SHIELD_SIZE.y, Image.INTERPOLATE_BILINEAR)
	return image


## Écu « français ancien » : rectangle jusqu'aux deux tiers, puis pointe arrondie.
static func _inside_shield(x: int, y: int) -> bool:
	var w := float(SHIELD_SIZE.x)
	var h := float(SHIELD_SIZE.y)
	var top := h * 0.62
	if y < top:
		return true
	var t := (float(y) - top) / (h - top)
	var half := (w * 0.5) * sqrt(maxf(0.0, 1.0 - t * t))
	return absf(float(x) + 0.5 - w * 0.5) <= half


static func _on_edge(x: int, y: int) -> bool:
	if x == 0 or y == 0 or x == SHIELD_SIZE.x - 1:
		return true
	for offset in [Vector2i(1, 0), Vector2i(-1, 0), Vector2i(0, 1), Vector2i(0, -1)]:
		if not _inside_shield(x + offset.x, y + offset.y):
			return true
	return false
