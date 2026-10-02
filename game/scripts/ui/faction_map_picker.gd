class_name FactionMapPicker
extends Control

## Lot FE6 (spec FE § 6) : choix de faction sur la carte de 1337 (les factions jouables ne
## tiennent plus en cartes). Chaque province est peinte aux couleurs de son propriétaire ; survol :
## fiche de la faction (souverain, titres, suzerain, royaume, difficulté, objectifs) ; clic :
## `faction_chosen`. Filtres par royaume et par rang (les autres factions sont voilées).
## Géométrie : `data/map/provinces.geojson` ; fiches : `GameDataStore.get_feudal_start_sheets`
## (déductions du cœur). Aucune règle ici.
## JR3 : une faction jouable sans province (les croisés, qui ne tiennent que Limassol) est posée
## sur la carte par une bannière à l'emplacement de sa colonie, cliquable comme une terre.

signal faction_chosen(faction_id: String)
signal faction_hovered(faction_id: String)

const SEA := Color(0.30, 0.38, 0.42)
const OUTLINE := Color(0.10, 0.07, 0.04, 0.55)
const VEIL := Color(0.18, 0.16, 0.14)
const SELECTED_OUTLINE := Color(0.98, 0.84, 0.36)

## Fiches par faction (`get_feudal_start_sheets`), dans l'ordre du cœur (souverains d'abord).
var sheets: Dictionary = {}
var sheet_order: PackedStringArray = PackedStringArray()
var selected: String = ""
var hovered: String = ""
var kingdom_filter: String = ""
var rank_filter: String = ""
## Provinces : `[{id, owner, polygons: [PackedVector2Array]}]` en coordonnées carte.
var provinces: Array = []
## JR3 : factions jouables sans province → position (coordonnées carte) de leur bannière.
var landless: Dictionary = {}
var _bounds := Rect2()
var _card: PanelContainer
var _card_text: RichTextLabel


func _init() -> void:
	name = "FactionMapPicker"
	mouse_filter = Control.MOUSE_FILTER_STOP
	clip_contents = true
	custom_minimum_size = Vector2(280, 240)
	_card = PanelContainer.new()
	_card.name = "HoverCard"
	_card.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_card.add_theme_stylebox_override("panel", FrontEndStyle.vellum_panel(FrontEndStyle.GOLD_DARK))
	_card.hide()
	# Hors du rognage de la carte : la fiche peut déborder sur la liste voisine.
	_card.top_level = true
	_card.z_index = 10
	add_child(_card)
	_card_text = RichTextLabel.new()
	_card_text.bbcode_enabled = true
	_card_text.fit_content = true
	_card_text.scroll_active = false
	_card_text.custom_minimum_size = Vector2(300, 0)
	_card_text.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_card_text.add_theme_color_override("default_color", FrontEndStyle.INK)
	UiType.apply(_card_text, UiType.CAPTION)
	_card.add_child(_card_text)


func _ready() -> void:
	load_data()


## Lit la géométrie et les fiches ; `false` si l'une manque (le choix par cartes reste possible).
func load_data(geojson_path: String = "") -> bool:
	var path := geojson_path if geojson_path != "" else FrontEndData._data_dir().path_join("map/provinces.geojson")
	provinces = read_provinces(path)
	var facade := _facade()
	var store: Object = facade.get("store") if facade != null else null
	sheets.clear()
	sheet_order = PackedStringArray()
	if store != null and store.has_method("get_feudal_start_sheets"):
		for sheet in store.call("get_feudal_start_sheets"):
			var id := str(sheet.get("id", ""))
			sheets[id] = sheet
			sheet_order.append(id)
			for province_id in sheet.get("provinces", PackedStringArray()):
				for province in provinces:
					if province["id"] == province_id:
						province["owner"] = id
	landless.clear()
	for id in sheet_order:
		if (sheets[id].get("provinces", PackedStringArray()) as PackedStringArray).is_empty():
			var home := home_position(id, store)
			if home != NO_POSITION:
				landless[id] = home
	_bounds = playable_bounds()
	queue_redraw()
	return not provinces.is_empty() and not sheets.is_empty()


## Marge autour de l'emprise des terres jouables (part de sa taille).
const FRAME_MARGIN := 0.04


## Emprise (coordonnées carte) des provinces des factions jouables, élargie de `FRAME_MARGIN` : la
## vue la remplit (les provinces lointaines de l'est et les bords de la carte sont coupés). À
## défaut de fiches, l'emprise de toutes les provinces.
func playable_bounds() -> Rect2:
	var bounds := Rect2()
	var first := true
	for pass_index in 2:
		for province in provinces:
			if pass_index == 0 and not sheets.has(str(province["owner"])):
				continue
			for polygon in province["polygons"]:
				for point in polygon:
					if first:
						bounds = Rect2(point, Vector2.ZERO)
						first = false
					else:
						bounds = bounds.expand(point)
		if not first:
			break
	for id in landless:
		bounds = bounds.expand(landless[id]) if not first else Rect2(landless[id], Vector2.ZERO)
		first = false
	return bounds.grow_individual(bounds.size.x * FRAME_MARGIN, bounds.size.y * FRAME_MARGIN,
		bounds.size.x * FRAME_MARGIN, bounds.size.y * FRAME_MARGIN)


static func read_provinces(path: String) -> Array:
	var result: Array = []
	var text := FileAccess.get_file_as_string(path)
	var parsed: Variant = JSON.parse_string(text) if text != "" else null
	if not (parsed is Dictionary):
		return result
	for feature in parsed.get("features", []):
		var props: Dictionary = feature.get("properties", {})
		var geometry: Dictionary = feature.get("geometry", {})
		var polygons: Array = []
		var coords: Array = geometry.get("coordinates", [])
		var parts: Array = [coords] if str(geometry.get("type", "")) == "Polygon" else coords
		for part in parts:
			if (part as Array).is_empty():
				continue
			var ring := PackedVector2Array()
			for point in part[0]:
				ring.append(Vector2(float(point[0]), float(point[1])))
			if ring.size() >= 3:
				polygons.append(ring)
		var entry := {"id": str(props.get("id", "")), "owner": str(props.get("owner", "")), "polygons": polygons}
		var seat: Variant = props.get("capital_px", props.get("centroid", null))
		if seat is Array and (seat as Array).size() >= 2:
			entry["seat"] = Vector2(float(seat[0]), float(seat[1]))
		result.append(entry)
	return result


const NO_POSITION := Vector2(-1, -1)
const SETTLEMENT_POSITIONS_FILE := "map/settlements_px.json"


## JR3 : où poser la bannière d'une faction sans province : la colonie qu'elle tient dans la
## province de sa capitale (`data/settlements/<capitale>.json`, position de
## `map/settlements_px.json`), à défaut le siège de cette province ; `NO_POSITION` sinon.
func home_position(faction_id: String, store: Object) -> Vector2:
	var info: Dictionary = store.call("get_faction", faction_id) if store != null and store.has_method("get_faction") else {}
	var capital := str(info.get("capital", ""))
	if capital == "":
		return NO_POSITION
	var data_dir := FrontEndData._data_dir()
	var listed: Variant = _read_json(data_dir.path_join("settlements/%s.json" % capital))
	if listed is Array:
		var positions: Variant = _read_json(data_dir.path_join(SETTLEMENT_POSITIONS_FILE))
		for settlement in listed:
			if not (settlement is Dictionary) or str(settlement.get("owner", "")) != faction_id:
				continue
			var px: Variant = (positions as Dictionary).get(str(settlement.get("id", "")), null) if positions is Dictionary else null
			if px is Array and (px as Array).size() >= 2:
				return Vector2(float(px[0]), float(px[1]))
	for province in provinces:
		if province["id"] == capital and province.has("seat"):
			return province["seat"]
	return NO_POSITION


static func _read_json(path: String) -> Variant:
	return JSON.parse_string(FileAccess.get_file_as_string(path)) if FileAccess.file_exists(path) else null


# --- Filtres ---------------------------------------------------------------------------------


func set_filters(kingdom: String, rank: String) -> void:
	kingdom_filter = kingdom
	rank_filter = rank
	queue_redraw()


func matches(faction_id: String) -> bool:
	var sheet: Dictionary = sheets.get(faction_id, {})
	if sheet.is_empty():
		return false
	if kingdom_filter != "" and str(sheet.get("kingdom", "")) != kingdom_filter:
		return false
	return rank_filter == "" or str(sheet.get("rank", "")) == rank_filter


## Royaumes présents (`[[titre, nom]]`), pour le filtre.
func kingdoms() -> Array:
	var seen := {}
	var result: Array = []
	for id in sheet_order:
		var sheet: Dictionary = sheets[id]
		var kingdom := str(sheet.get("kingdom", ""))
		if kingdom != "" and not seen.has(kingdom):
			seen[kingdom] = true
			result.append([kingdom, str(sheet.get("kingdom_name", kingdom))])
	result.sort_custom(func(a: Array, b: Array) -> bool: return str(a[1]) < str(b[1]))
	return result


## Factions jouables qui passent les filtres, dans l'ordre du cœur.
func visible_factions() -> PackedStringArray:
	var result := PackedStringArray()
	for id in sheet_order:
		if matches(id):
			result.append(id)
	return result


## Largeur / hauteur de l'emprise cadrée (pour dimensionner la vue sans bandes de mer).
func aspect_ratio() -> float:
	return _bounds.size.x / _bounds.size.y if _bounds.size.y > 0.0 else 1.0


## Surbrillance d'une faction depuis l'extérieur (liste), sans fiche de survol.
func highlight(faction_id: String) -> void:
	if faction_id != hovered:
		hovered = faction_id
		queue_redraw()


func select(faction_id: String) -> void:
	selected = faction_id
	queue_redraw()


# --- Géométrie écran ---------------------------------------------------------------------------


func _transform() -> Transform2D:
	if _bounds.size.x <= 0.0 or _bounds.size.y <= 0.0:
		return Transform2D.IDENTITY
	var scale_factor := minf(size.x / _bounds.size.x, size.y / _bounds.size.y)
	var offset := (size - _bounds.size * scale_factor) * 0.5 - _bounds.position * scale_factor
	return Transform2D(0.0, Vector2(scale_factor, scale_factor), 0.0, offset)


## Faction propriétaire de la province sous `local` ("" hors des terres jouables).
func faction_at(local: Vector2) -> String:
	var xform := _transform()
	for id in landless:  # JR3 : la bannière passe avant la terre qu'elle recouvre
		if local.distance_to(xform * (landless[id] as Vector2)) <= BANNER_RADIUS + BANNER_PICK_MARGIN:
			return id
	var map_point := xform.affine_inverse() * local
	for province in provinces:
		for polygon in province["polygons"]:
			if Geometry2D.is_point_in_polygon(map_point, polygon):
				var owner := str(province["owner"])
				return owner if sheets.has(owner) else ""
	return ""


## Point (coordonnées locales) à l'intérieur d'une terre de `faction_id` : centre du plus grand
## triangle de sa plus grande province (tests, captures) ; (-1, -1) si elle n'en a aucune.
func faction_center(faction_id: String) -> Vector2:
	if landless.has(faction_id):
		return _transform() * (landless[faction_id] as Vector2)
	var best := Vector2(-1, -1)
	var best_area := 0.0
	for province in provinces:
		if province["owner"] != faction_id:
			continue
		for polygon in province["polygons"]:
			var points: PackedVector2Array = polygon
			var triangles := Geometry2D.triangulate_polygon(points)
			for i in range(0, triangles.size(), 3):
				var a := points[triangles[i]]
				var b := points[triangles[i + 1]]
				var c := points[triangles[i + 2]]
				var area := absf((b - a).cross(c - a)) * 0.5
				if area > best_area:
					best_area = area
					best = (a + b + c) / 3.0
	return _transform() * best if best_area > 0.0 else Vector2(-1, -1)


## L'autoload `SimFacade` (lu par l'arbre même avant l'entrée de ce contrôle dans la scène).
static func _facade() -> Node:
	var tree := Engine.get_main_loop() as SceneTree
	return tree.root.get_node_or_null("SimFacade") if tree != null else null


func _faction_color(faction_id: String) -> Color:
	var facade := _facade()
	return facade.call("faction_color", faction_id) if facade != null else Color(0.5, 0.5, 0.5)


func _draw() -> void:
	draw_rect(Rect2(Vector2.ZERO, size), SEA)
	var xform := _transform()
	for province in provinces:
		var owner := str(province["owner"])
		var color := _faction_color(owner) if owner != "" else VEIL
		if not sheets.has(owner):
			color = color.lerp(VEIL, 0.7)  # faction non jouable
		elif not matches(owner):
			color = color.lerp(VEIL, 0.6)
		if owner == hovered and owner != "":
			color = color.lightened(0.25)
		for polygon in province["polygons"]:
			var screen := xform * (polygon as PackedVector2Array)
			if Geometry2D.triangulate_polygon(screen).is_empty():
				continue
			draw_colored_polygon(screen, color)
			var closed := screen.duplicate()
			closed.append(screen[0])
			var outline := SELECTED_OUTLINE if owner == selected and owner != "" else OUTLINE
			draw_polyline(closed, outline, 2.0 if owner == selected else 1.0, true)
	for id in landless:
		_draw_banner(str(id), xform * (landless[id] as Vector2))


## Rayon (px écran) du disque de la bannière d'une faction sans province, et marge de clic.
const BANNER_RADIUS := 7.0
const BANNER_PICK_MARGIN := 4.0


## JR3 : bannière d'une faction sans province : hampe et pennon aux couleurs de la faction sur
## un disque cerné, pour qu'elle se lise par-dessus la terre d'autrui où elle campe.
func _draw_banner(faction_id: String, at: Vector2) -> void:
	var color := _faction_color(faction_id)
	if not matches(faction_id):
		color = color.lerp(VEIL, 0.6)
	if faction_id == hovered:
		color = color.lightened(0.25)
	var chosen := faction_id == selected
	var ring := SELECTED_OUTLINE if chosen else FrontEndStyle.GOLD
	var dark := Color(OUTLINE, 1.0)
	draw_circle(at, BANNER_RADIUS + (2.5 if chosen else 1.5), dark)
	draw_circle(at, BANNER_RADIUS, color)
	draw_arc(at, BANNER_RADIUS, 0.0, TAU, 24, ring, 2.0 if chosen else 1.2, true)
	var foot := at + Vector2(0, -BANNER_RADIUS)
	var top := foot + Vector2(0, -BANNER_RADIUS * 1.9)
	draw_line(foot, top, dark, 1.5, true)
	var pennon := PackedVector2Array([top, top + Vector2(BANNER_RADIUS * 1.6, BANNER_RADIUS * 0.45), top + Vector2(0, BANNER_RADIUS * 0.9)])
	draw_colored_polygon(pennon, color)
	draw_polyline(pennon + PackedVector2Array([top]), ring, 1.0, true)


func _gui_input(event: InputEvent) -> void:
	if event is InputEventMouseMotion:
		_hover(faction_at(event.position), event.position)
	elif event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
		var faction := faction_at(event.position)
		if faction != "":
			select(faction)
			faction_chosen.emit(faction)
			accept_event()


func _notification(what: int) -> void:
	if what == NOTIFICATION_MOUSE_EXIT:
		_hover("", Vector2.ZERO)
	elif what == NOTIFICATION_RESIZED:
		queue_redraw()


func _hover(faction_id: String, at: Vector2) -> void:
	if faction_id != hovered:
		hovered = faction_id
		faction_hovered.emit(faction_id)
		queue_redraw()
	if faction_id == "":
		_card.hide()
		return
	_card_text.text = card_text(faction_id)
	_card.show()
	_card.reset_size()
	var screen := get_viewport_rect().size
	var pos := get_global_transform() * at + Vector2(18, 18)
	pos.x = minf(pos.x, screen.x - _card.size.x - 4)
	pos.y = minf(pos.y, screen.y - _card.size.y - 4)
	_card.global_position = pos.max(Vector2(4, 4))


## Fiche de survol : souverain, titres, suzerain, royaume, difficulté, objectifs.
func card_text(faction_id: String) -> String:
	var sheet: Dictionary = sheets.get(faction_id, {})
	if sheet.is_empty():
		return ""
	var lines := PackedStringArray()
	lines.append("[b]%s[/b]" % str(sheet.get("name", faction_id)))
	var ruler := str(sheet.get("ruler", ""))
	if ruler != "":
		lines.append("Souverain : %s" % ruler)
	var titles := PackedStringArray()
	for title in sheet.get("titles", []):
		titles.append(str(title.get("name", "")))
	# JR3 : une faction sans terre n'a ni titre ni royaume.
	lines.append("Titres : %s" % (", ".join(titles) if not titles.is_empty() else "aucun (ost sans terre)"))
	var liege := str(sheet.get("liege_name", ""))
	lines.append("Suzerain : %s" % liege if liege != "" else "Suzerain : aucun (souverain)")
	if str(sheet.get("kingdom_name", "")) != "":
		lines.append("Royaume : %s" % str(sheet.get("kingdom_name", "")))
	var entry := FrontEndData.faction(faction_id)
	if entry.has("difficulty"):
		lines.append("Difficulté estimée : %s" % FrontEndData.difficulty_label(int(entry["difficulty"])))
	var objectives := PackedStringArray()
	for objective in sheet.get("objectives", []):
		objectives.append(str(objective.get("title", "")))
	if not objectives.is_empty():
		lines.append("Objectifs : %s" % ", ".join(objectives))
	return "\n".join(lines)
