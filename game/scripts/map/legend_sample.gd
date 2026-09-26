class_name LegendSample
extends Control

## Lot UX1 : échantillon dessiné d'un symbole de la carte pour la légende (`MapLegend`).
## Les symboles reprennent le vrai rendu : marqueurs peints de l'atlas du lot DA3
## (`SettlementMarkers`, écu du royaume composé comme dans `settlement_icon.gdshader`), plaque d'effectif (`ArmyMarkers.build_plate`), jeton d'agent
## (`AgentController.Token`), étendard de faction (`ArmyMarker.standard_for`), couleurs de
## relation (`MapModeController.RELATION_COLORS`), anneaux et chemins aux couleurs des couches.
## `build(sample, context)` renvoie le contrôle adapté ; `context` : {player_color: Color,
## player_faction: String, factions: [[id, Color, nom]]}.

const SIZE := Vector2(96, 30)
const AGENT_SCRIPT := "res://scripts/map/agent_controller.gd"
const NEUTRAL := Color(0.62, 0.6, 0.55)  # colonie sans contrôleur (`SettlementLayer.refresh`)
## Couleurs des couches (reprises de leurs scripts : `ArmyMovementPath`, `TradeRouteLayer`,
## `ConstructionMarkers`, `CampaignMinimap`).
const PATH_NOW := Color(0.40, 0.88, 0.32, 1.0)
const PATH_LATER := Color(0.92, 0.36, 0.20, 0.95)
const PATH_MARK := Color(1.0, 0.90, 0.60, 1.0)
const TRADE := Color(0.45, 0.18, 0.08, 0.9)
const TRADE_CUT := Color(0.4, 0.4, 0.4, 0.6)
const HAMMER := Color(0.35, 0.22, 0.08)
const LAND := Color(0.56, 0.54, 0.36)
const FOG_MIST := Color(0.43, 0.42, 0.39)

var sample: Dictionary = {}
var context: Dictionary = {}
## Catalogue des marqueurs (lot DA3), chargé une fois.
static var _markers: SettlementMarkers


## Contrôle d'échantillon pour une entrée `sample` de `data/ui/map_legend.json`.
static func build(sample_data: Dictionary, legend_context: Dictionary) -> Control:
	var type := str(sample_data.get("type", ""))
	var holder := CenterContainer.new()
	holder.custom_minimum_size = SIZE
	holder.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var child: Control = null
	match type:
		"army_plate":
			child = _plate(sample_data, legend_context)
		"agent":
			# Chargé à l'exécution : `agent_controller.gd` nomme l'autoload SimFacade, inconnu
			# à la compilation des tests headless (`--script`).
			var token: Control = load(AGENT_SCRIPT).Token.new()
			token.set("kind", str(sample_data.get("kind", "spy")))
			token.set("level", 2)
			token.set("faction_color", legend_context.get("player_color", Color.WHITE))
			child = token
		"army_banner":
			child = _banner(legend_context)
		"construction":
			var label := Label.new()
			label.text = "⚒"
			label.add_theme_font_size_override("font_size", 22)
			label.add_theme_color_override("font_color", HAMMER)
			label.add_theme_color_override("font_outline_color", Color(0.97, 0.92, 0.80))
			label.add_theme_constant_override("outline_size", 6)
			child = label
		"faction_colors":
			child = _swatch_row(legend_context)
		_:
			var drawn := LegendSample.new()
			drawn.sample = sample_data
			drawn.context = legend_context
			drawn.custom_minimum_size = SIZE
			child = drawn
	if child != null:
		child.mouse_filter = Control.MOUSE_FILTER_IGNORE
		holder.add_child(child)
	holder.name = "Sample_" + type
	return holder


static func _plate(sample_data: Dictionary, legend_context: Dictionary) -> Control:
	var marker := ArmyMarker.new()
	var player := str(sample_data.get("owner", "player")) == "player"
	marker.is_player = player
	marker.men = int(sample_data.get("men", 0))
	marker.unit_count = 1
	marker.status = str(sample_data.get("status", ""))
	if player:
		marker.faction_id = str(legend_context.get("player_faction", ""))
		marker.faction_color = legend_context.get("player_color", Color.WHITE)
	else:
		var others: Array = legend_context.get("factions", [])
		if not others.is_empty():
			marker.faction_id = str(others[0][0])
			marker.faction_color = others[0][1]
	marker.army_id = "legend"
	var plate := ArmyMarkers.build_plate(marker, false)
	marker.free()
	return plate


static func _banner(legend_context: Dictionary) -> Control:
	var faction := str(legend_context.get("player_faction", ""))
	var standard := ArmyMarker.standard_for(faction, {})
	var texture: Texture2D = standard.get("texture")
	if texture == null:
		var swatch := ColorRect.new()
		swatch.color = legend_context.get("player_color", Color.WHITE)
		swatch.custom_minimum_size = Vector2(14, 22)
		return swatch
	var rect := TextureRect.new()
	rect.texture = texture
	rect.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	rect.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	rect.custom_minimum_size = Vector2(26, 44)
	return rect


static func _swatch_row(legend_context: Dictionary) -> Control:
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 2)
	var entries: Array = [[legend_context.get("player_faction", ""), legend_context.get("player_color", Color.WHITE), "Vous"]]
	entries.append_array(legend_context.get("factions", []))
	for entry in entries.slice(0, 5):
		var swatch := ColorRect.new()
		swatch.color = entry[1]
		swatch.custom_minimum_size = Vector2(11, 18)
		swatch.tooltip_text = str(entry[2])
		row.add_child(swatch)
	return row


## Faction d'une place selon `owner` (joueur, autre royaume) ; "" sans seigneur.
func _owner_faction(owner: String) -> String:
	if owner == "none":
		return ""
	if owner == "other":
		var others: Array = context.get("factions", [])
		return str(others[0][0]) if not others.is_empty() else ""
	return str(context.get("player_faction", ""))


func _draw() -> void:
	var center := size * 0.5
	match str(sample.get("type", "")):
		"settlement":
			var markers := catalog()
			var pictogram := str(sample.get("pictogram", markers.pictogram_for(str(sample.get("kind", "village")), int(sample.get("rank", 2)))))
			draw_marker(self, pictogram, center, 16.0, _owner_faction(str(sample.get("owner", "player"))), bool(sample.get("port", false)))
		"settlement_owners":
			var factions: Array = [str(context.get("player_faction", ""))]
			for entry in (context.get("factions", []) as Array).slice(0, 2):
				factions.append(str(entry[0]))
			for i in factions.size():
				draw_marker(self, "city", Vector2(14.0 + i * 24.0, center.y), 12.0, factions[i])
		"relation":
			# DP2 : positions diplomatiques (allié, accord, neutre, tension...) d'abord.
			var relation := str(sample.get("relation", ""))
			_draw_swatch(DiplomaticStances.COLORS.get(relation, MapModeController.RELATION_COLORS.get(relation, NEUTRAL)))
		"color":
			_draw_swatch(Color.html(str(sample.get("color", "#808080"))))
		"gradient":
			var from := Color.html(str(sample.get("from", "#338c33")))
			var to := Color.html(str(sample.get("to", "#bf261a")))
			var steps := 12
			var width := 48.0 / steps
			for i in steps:
				draw_rect(Rect2(Vector2(center.x - 24.0 + i * width, center.y - 8.0), Vector2(width + 0.5, 16.0)), from.lerp(to, float(i) / (steps - 1)))
			draw_rect(Rect2(center - Vector2(24, 8), Vector2(48, 16)), HudStyle.INK, false, 1.0)
		"fog":
			var rect := Rect2(center - Vector2(26, 10), Vector2(52, 20))
			draw_rect(rect, LAND)
			draw_rect(Rect2(Vector2(center.x, rect.position.y), Vector2(26, 20)), FOG_MIST)
			draw_circle(center + Vector2(9, -3), 5.0, Color(0.62, 0.61, 0.58, 0.7))
			draw_circle(center + Vector2(17, 3), 6.0, Color(0.62, 0.61, 0.58, 0.6))
			draw_line(Vector2(center.x, rect.position.y), Vector2(center.x, rect.end.y), HudStyle.INK_FADED, 1.0)
			draw_rect(rect, HudStyle.INK, false, 1.0)
		"path":
			var a := center + Vector2(-26, 6)
			var b := center + Vector2(0, -2)
			var c := center + Vector2(26, 4)
			draw_line(a, b, PATH_NOW, 4.0, true)
			draw_line(b, c, PATH_LATER, 3.2, true)
			draw_circle(b, 3.5, PATH_MARK)
			draw_circle(b, 3.5, HudStyle.INK, false, 1.0)
		"ring":
			var color := Color.html(str(sample.get("color", "#ffffff")))
			draw_arc(center, 10.0, 0.0, TAU, 40, HudStyle.INK.lerp(color, 0.2), 5.0, true)
			draw_arc(center, 10.0, 0.0, TAU, 40, color, 3.0, true)
		"trade_route":
			draw_line(center + Vector2(-26, -4), center + Vector2(26, -4), TRADE, 3.0, true)
			draw_dashed_line(center + Vector2(-26, 6), center + Vector2(26, 6), TRADE_CUT, 2.0, 4.0)
		"minimap_army":
			draw_rect(Rect2(center - Vector2(14, 11), Vector2(28, 22)), CampaignMinimap.LOWLAND)
			if str(sample.get("owner", "player")) == "player":
				draw_circle(center, 4.5, CampaignMinimap.PLAYER_RING)
				draw_circle(center, 3.0, context.get("player_color", NEUTRAL))
			else:
				var others: Array = context.get("factions", [])
				var color: Color = others[0][1] if not others.is_empty() else NEUTRAL
				draw_colored_polygon(PackedVector2Array([center + Vector2(0, -4), center + Vector2(4, 0), center + Vector2(0, 4), center + Vector2(-4, 0)]), HudStyle.INK)
				draw_colored_polygon(PackedVector2Array([center + Vector2(0, -2.5), center + Vector2(2.5, 0), center + Vector2(0, 2.5), center + Vector2(-2.5, 0)]), color)
		"minimap_frame":
			var rect := Rect2(center - Vector2(26, 12), Vector2(52, 24))
			draw_rect(rect, CampaignMinimap.SEA)
			draw_rect(Rect2(rect.position, Vector2(30, 24)), CampaignMinimap.LOWLAND)
			var frame := PackedVector2Array([center + Vector2(-16, -8), center + Vector2(16, -8), center + Vector2(10, 8), center + Vector2(-10, 8), center + Vector2(-16, -8)])
			draw_polyline(frame, HudStyle.INK, 3.0)
			draw_polyline(frame, CampaignMinimap.FRAME_COLOR, 1.5)


func _draw_swatch(color: Color) -> void:
	var rect := Rect2(size * 0.5 - Vector2(18, 9), Vector2(36, 18))
	draw_rect(rect, color)
	draw_rect(rect, HudStyle.INK, false, 1.0)


## Lot DA3 : catalogue des marqueurs de lieux (chargé une fois).
static func catalog() -> SettlementMarkers:
	if _markers == null:
		_markers = SettlementMarkers.load_default()
	return _markers


## Marqueur de lieu (lot DA3) : pictogramme peint de l'atlas, insigne de port, écu du royaume
## `faction` ("" : sans écu), placés comme dans `settlement_icon.gdshader`. `half` : demi-taille
## en pixels ; la base du pictogramme est sous `center`.
static func draw_marker(canvas: CanvasItem, pictogram: String, center: Vector2, half: float, faction: String, port: bool = false) -> void:
	var markers := catalog()
	if markers.atlas == null:
		return
	var box := Rect2(center - Vector2(half, half), Vector2(half, half) * 2.0)
	var atlas_size := markers.atlas.get_size()
	var cell := markers.cell_of(pictogram)
	if cell >= 0:
		var region := markers.cell_uv_rect(cell)
		canvas.draw_texture_rect_region(markers.atlas, box, Rect2(region.position * atlas_size, region.size * atlas_size))
	if port and markers.port_cell() >= 0:
		var badge := _placed(box, markers.placement("badge"))
		var badge_region := markers.cell_uv_rect(markers.port_cell())
		canvas.draw_texture_rect_region(markers.atlas, badge, Rect2(badge_region.position * atlas_size, badge_region.size * atlas_size))
	var shield := PortraitLoader.heraldry_texture(faction)
	if shield != null:
		canvas.draw_texture_rect(shield, _placed(box, markers.placement("shield")), false)


## Rectangle écran d'un élément placé (centre et demi-taille en fraction, origine en bas à gauche).
static func _placed(box: Rect2, place: Vector3) -> Rect2:
	var center := box.position + Vector2(place.x, 1.0 - place.y) * box.size
	var half := place.z * box.size
	return Rect2(center - half, half * 2.0)
