class_name SettlementLayer
extends Node3D

## Colonies et hameaux sur la carte de campagne (lot C6), rendu seulement. Lot DV2 (ADR 0124) :
## tout vit dans la vue normale (`normal = 1 − ZoomTiers.strategic_weight`), le parchemin prend le
## relais au-delà ; plus de marqueur peint.
## VT (ADR 0138) : plus de maquette de colonie ni de ville emblématique sur la carte ; les villes
## sont dessinées à l'échelle 1:1 à toutes les hauteurs (`TownLayer`, `LandmarkCityLayer`, tuiles
## lointaines). Ce calque garde l'emprise réelle de chaque colonie (`radii` de `towns_1340.json`) :
## clic, anneau de sélection, étiquettes, exclusions (hameaux, végétation par le finage).
## - nom de chaque lieu au-dessus de son emprise, surmonté d'un petit écu du détenteur
##   (`settlement_icon.gdshader`, un `MultiMesh`, atlas `HeraldryAtlas`) ; taille selon le rang et
##   densité par rang et distance caméra (`SettlementMarkers`, données) ; estompés par `normal` ;
## - détail proche (`ZoomTiers.near_weight`) : hameaux en `MultiMesh` par tuile (orientation et
##   variante déterministes, brûlés selon la dévastation de la province).
## Dé-encombrement écran (lot DA7d, ADR 0066) : couples nom + écu posés par priorité (rang puis
## poids, `MarkerDeclutter`) ; ceux qui recouvrent un rectangle déjà posé cèdent la place (fondu
## du shader), sauf la capitale du joueur, la colonie sélectionnée et celle survolée. Recalcul
## seulement quand la caméra bouge nettement (paramètres dans `settlement_markers.json`).
## Picking écran : `pick_screen` → id, `select` → surbrillance + signal.

signal settlement_selected(id: String)

## Lot PO3 (bible DA § 4, § 12.2) : noms dans le registre manuscrit — EB Garamond, graisse et taille
## selon le rang (cité `Heading`, ville `Body`, bourg `Caption`), encre sombre sur un halo de
## parchemin léger (plus de pastille claire).
## Tailles de base de `UiType` (PO2) : Label3D n'a pas de variation de type de thème.
const LABEL_TYPE := {"city": UiType.HEADING, "town": UiType.BODY, "castle": UiType.CAPTION, "abbey": UiType.CAPTION, "village": UiType.CAPTION}
const LABEL_WEIGHT := {"city": 700, "town": 600, "castle": 500, "abbey": 500, "village": 500}
const LABEL_FONT_PATH := "res://assets/third_party/fonts/eb_garamond/EBGaramond-VariableFont_wght.ttf"
## Halo : contour fin (px de police) et part d'opacité du halo.
const LABEL_OUTLINE_PX := 4
const LABEL_HALO_ALPHA := 0.6
## Rayon de picking d'un écu, en fraction de sa taille écran.
const PICK_ICON_FRACTION := 0.6
## Nom d'un lieu : écart (px écran) entre le haut de son emprise et le bas du texte mesuré (DC4).
const LABEL_GAP_PX := 2.0
## VT : décalage du nom au-dessus de l'emprise projetée, plafonné (px écran) pour qu'une grande
## ville vue de près ne chasse pas son nom hors de l'écran.
const LABEL_FOOTPRINT_MAX_PX := 160.0
## VT : rayon de clic minimal (px écran) d'une emprise.
const PICK_MIN_PX := 8.0
## VT : emprise (m) d'une colonie absente de `towns_1340.json` ; hauteur (m) des toits au-dessus
## du sol (`model_top`).
const DEFAULT_FOOTPRINT_M := 150.0
const TOWN_TOP_M := 12.0
## Proportion de hameaux brûlés = dévastation (%) × ce facteur (au-delà d'un seuil).
const BURN_THRESHOLD := 10.0
## Distance de retrait du marqueur de la colonie sélectionnée (toujours affiché, DA7d).
const SELECTED_UNTIL := 100000.0

@export var tiers: ZoomTiers
## Échelle globale des écus (tailles par rang dans `data/map/settlement_markers.json`).
@export var icon_size_scale: float = 1.0
@export var label_color: Color = Color(0.13, 0.085, 0.045)
@export var label_outline: Color = Color(0.93, 0.87, 0.72)
@export var declutter_interval: float = 0.15
@export var declutter_margin: float = 3.0
@export var max_hamlet_builds_per_frame: int = 4

var map_data: MapData
var terrain: TerrainBuilder
var data: SettlementData
var selected_id: String = ""
var stats: Dictionary = {}
## CV3-0 (#7) : rectangles écran réservés (plaques/étendards d'armée) que le déclutter des
## colonies doit éviter ; posé par `CampaignMap` (`ArmyMarkers.screen_label_rects`).
var label_obstacles: Callable = Callable()

var _icons: MultiMeshInstance3D
var _icon_material: ShaderMaterial
## Lot DA3 : catalogue des marqueurs ; par colonie : rang, taille écran de l'écu (px), distance de
## retrait.
var markers: SettlementMarkers
## Lot DV2 : atlas des écus des détenteurs.
var heraldry := HeraldryAtlas.new()
var _marker_rank: PackedInt32Array = PackedInt32Array()
var _marker_size: PackedFloat32Array = PackedFloat32Array()
var _marker_until: PackedFloat32Array = PackedFloat32Array()
## DC4, DV2 : ancre monde de chaque couple nom + écu (= position du `Label3D`), ancre du
## `MultiMesh` (picking et dé-encombrement DA7d sans relire le relief).
var _marker_world: PackedVector3Array = PackedVector3Array()
## Écu affiché par colonie (faction), pour ne réécrire que ce qui change ; 1 si armorié (DV2).
var _marker_holder: PackedStringArray = PackedStringArray()
var _shielded: PackedByteArray = PackedByteArray()
## Lot DA7d : état de dé-encombrement par colonie (1 = marqueur affiché, 0 = cède la place),
## ordre de priorité fixe (rang puis poids), épinglés et dernier état de caméra calculé.
var _marker_shown: PackedByteArray = PackedByteArray()
var _priority_order: PackedInt32Array = PackedInt32Array()
var _placer := MarkerDeclutter.new()
var _capital_index := -1
var _hovered_index := -1
var _selected_index := -1
var _declutter_force := true
var _declutter_camera: Array = []
var _declutter_fade_until := -1.0
var _max_marker_px := 64.0
## Centre écran de chaque marqueur affiché au dernier recalcul (survol sans reprojection).
var _marker_screen: PackedVector2Array = PackedVector2Array()
## Durée (ms) du dernier recalcul complet (`declutter`).
var last_declutter_ms := 0.0
var _icon_distance := -1.0
var _labels: Array[Label3D] = []
## DC4 : taille du texte de chaque étiquette (police, contour compris), mesurée à la demande.
var _label_size: PackedVector2Array = PackedVector2Array()
var _label_kind: PackedStringArray = PackedStringArray()
## VT : par colonie, emprise réelle (unités monde : plus grand des `radii` de `towns_1340.json`,
## sinon `DEFAULT_FOOTPRINT_M`), hauteur des toits au-dessus du sol (unités), rayon du finage
## (unités, < 0 si inconnu) et sol affiché au centre de l'emprise (recalé avec le relief).
var _model_radius: PackedFloat32Array = PackedFloat32Array()
var _model_top: PackedFloat32Array = PackedFloat32Array()
var _finage_radius: PackedFloat32Array = PackedFloat32Array()
var _ground_y: PackedFloat32Array = PackedFloat32Array()
## Villes emblématiques (lot L1) : index de colonie → cercle (x, z, rayon de zone) lu dans
## `data/landmarks/` sans instancier de maquette (zones caméra et armées, hameaux).
var _landmarks: Dictionary = {}
var _labels_dirty: bool = false
var _labels_root: Node3D
var _hamlets_root: Node3D
var _selection_ring: MeshInstance3D
## Q8 : l'anneau de sélection disparaît quand la caméra est à moins de N rayons de la ville.
const RING_HIDE_DISTANCE_FACTOR := 4.0
var _settlements_by_chunk: Dictionary = {}
var _hamlets_by_chunk: Dictionary = {}
var _hamlet_nodes: Dictionary = {}  # index de tuile → Node3D
var _hamlet_dirty: Dictionary = {}
var _burned_material: StandardMaterial3D
## Dévastation par province (0-100), lue depuis la simulation au rafraîchissement.
var _devastation: Dictionary = {}
var _declutter_timer := 0.0
var _weights := Vector3(-1, -1, -1)  # DV2 : détail proche, vue normale, vue stratégique
## DV2 : échelle écran des `Label3D` transmise au shader des écus (px écran par px d'étiquette).
var _shield_label_scale := -1.0
## DV2 : fondu de retrait par rang et écart nom → écu (données, lus une fois).
var _fade_distance := 60.0
var _shield_gap := 1.0
var _camera_distance := 1000.0
## ZG6 : villes ordinaires à l'échelle réelle (paliers vallée et site), voir `TownLayer`.
var towns: TownLayer
## VH4 (ADR 0078) : villes emblématiques à l'échelle 1:1 (format v2), voir `LandmarkCityLayer`.
var landmark_cities: LandmarkCityLayer
## VT-E (ADR 0138) : lointain des villes à l'échelle 1:1 (tuiles F1/F2), voir `TownFarLayer`.
var town_far: TownFarLayer
## Lot ZG5b : positions de rendu affinées (`fine_anchors.json`) des maquettes (index → Vector2)
## et des hameaux (x, y, z, déplacement), sans toucher aux positions de règles (`data`).
var _anchor_px: Dictionary = {}
var _hamlet_anchors: PackedVector4Array = PackedVector4Array()
## Lot SZ4 : échelle appliquée aux hameaux (1 au loin, taille réelle au palier vallée,
## `MapPropScale.hamlet_scale`) ; les tuiles sont reconstruites par pas de `rewrite_step`.
var _hamlet_scale := 1.0
## VT : rayon réel au sol (unités, < 0 si la colonie est absente de `towns_1340.json`).
var _real_radius: PackedFloat32Array = PackedFloat32Array()
## RS-K2 : hameaux écartés (ville emblématique, emprise d'une colonie) mémorisés : les tuiles de
## hameaux sont reconstruites à chaque recalage de relief, le test ne dépend que des emprises et des
## ancrages.
## 0 : à calculer, 1 : posé, 2 : écarté ; graine de tirage par hameau (0 : à calculer).
var _hamlet_keep: PackedByteArray = PackedByteArray()
var _hamlet_seed: PackedInt64Array = PackedInt64Array()
## RS-K3 : partie fixe d'une tuile de hameaux (morceau → [hameaux posés, points des hauteurs (5 par
## hameau : centre puis emprise), lacets, échelles]) : une reconstruction (recalage du relief,
## dévastation) ne relit plus que les hauteurs. Vidée avec les exclusions.
var _hamlet_tiles: Dictionary = {}
## RS-K3, VT : emprises des colonies ordinaires autour d'un morceau (3 × 3 morceaux) : morceau →
## [centres, rayons + marge] ; même validité que les exclusions des hameaux.
var _model_disks: Dictionary = {}
## RS-K2 : échelle écran pour les décalages d'étiquettes, par image.
var _lift_frame := -1
var _lift_scale := 1.0
var _lift_focal := 0.0
var _lift_eye := Vector3.ZERO


func setup(map: MapData, terrain_builder: TerrainBuilder, settlement_data: SettlementData, zoom_tiers: ZoomTiers) -> void:
	for child in get_children():
		child.queue_free()
	map_data = map
	terrain = terrain_builder
	data = settlement_data
	tiers = zoom_tiers if zoom_tiers != null else ZoomTiers.new()
	_labels.clear()
	_label_size.clear()
	_label_kind.clear()
	_settlements_by_chunk.clear()
	_hamlets_by_chunk.clear()
	_hamlet_nodes.clear()
	_anchor_px.clear()
	_labels_placed = false
	_burned_material = StandardMaterial3D.new()
	_burned_material.albedo_color = Color(0.075, 0.062, 0.05)
	_burned_material.roughness = 1.0
	_labels_root = Node3D.new()
	_labels_root.name = "Labels"
	add_child(_labels_root)
	_hamlets_root = Node3D.new()
	_hamlets_root.name = "Hamlets"
	add_child(_hamlets_root)
	var count := data.settlements.size()
	_model_radius.resize(count)
	_model_top.resize(count)
	_finage_radius.resize(count)
	_ground_y.resize(count)
	_real_radius.resize(count)
	_real_radius.fill(-1.0)
	_load_landmark_zones()
	for i in count:
		var entry: Dictionary = data.settlements[i]
		var px: Vector2 = entry["px"]
		_register(_settlements_by_chunk, terrain.chunk_index_at(px.x, px.y), i)
		_build_label(i, entry)
	for i in data.hamlets.size():
		var hpx: Vector2 = data.hamlets[i]["px"]
		_register(_hamlets_by_chunk, terrain.chunk_index_at(hpx.x, hpx.y), i)
	_hamlet_seed.resize(data.hamlets.size())
	_hamlet_seed.fill(-1)
	# RS-K3 : maillages des hameaux chargés au chargement de la carte (~15 ms la première fois),
	# pas à la construction de la première tuile en jeu.
	if not data.hamlets.is_empty():
		ModelLibrary.hamlet_meshes()
	_build_icons()
	_build_selection_ring()
	_setup_towns()  # VT : emprises réelles (`_compute_footprints`) et exclusions des hameaux
	if not terrain.chunk_surface_changed.is_connected(_on_chunk_surface_changed):
		terrain.chunk_surface_changed.connect(_on_chunk_surface_changed)
	stats = {"settlements": count, "hamlets": data.hamlets.size(), "footprints": _count_footprints()}


static func _register(map: Dictionary, key: int, value: int) -> void:
	if key < 0:
		return
	if not map.has(key):
		map[key] = PackedInt32Array()
	map[key].append(value)


static func _hash(text: String) -> int:
	return absi(text.hash())


# --- Construction ------------------------------------------------------------------


## VT : cercles (x, z, rayon de zone) des villes emblématiques L1, lus dans `data/landmarks/`
## (`scale.zone_radius_px`, `anchor.px`) sans instancier de `LandmarkModel` (la maquette ne sert
## plus qu'en bataille).
func _load_landmark_zones() -> void:
	_landmarks.clear()
	for i in data.settlements.size():
		var plan := LandmarkLibrary.for_settlement(str(data.settlements[i]["id"]))
		if plan.is_empty():
			continue
		var anchor: Array = (plan.get("anchor", {}) as Dictionary).get("px", [])
		if anchor.size() < 2:
			continue
		var zone := float((plan.get("scale", {}) as Dictionary).get("zone_radius_px", 6.0))
		_landmarks[i] = Vector3(float(anchor[0]), float(anchor[1]), zone)


## VT : emprise réelle de chaque colonie (unités monde) : plus grand des 32 `radii` de
## `towns_1340.json` (sans gain), finage (`finage_radius_m`) et hauteur des toits ; colonie
## absente : `DEFAULT_FOOTPRINT_M`, sans finage.
func _compute_footprints() -> void:
	var town_data: TownData = towns.data if towns != null else null
	var mpu := town_data.meters_per_unit if town_data != null else 719.0
	for i in data.settlements.size():
		var id := str(data.settlements[i]["id"])
		var radius_m := -1.0
		var finage_m := -1.0
		if town_data != null and town_data.has_town(id):
			var town: Dictionary = town_data.towns[id]
			for v in town.get("radii", []):
				radius_m = maxf(radius_m, float(v))
			finage_m = float(town.get("finage_radius_m", -1.0))
		_real_radius[i] = radius_m / mpu if radius_m > 0.0 else -1.0
		_model_radius[i] = (radius_m if radius_m > 0.0 else DEFAULT_FOOTPRINT_M) / mpu
		_finage_radius[i] = finage_m / mpu if finage_m > 0.0 else -1.0
		_model_top[i] = TOWN_TOP_M / mpu
		_ground_footprint(i)
	_forget_hamlet_exclusions()


func _count_footprints() -> int:
	var known := 0
	for r in _real_radius:
		if r > 0.0:
			known += 1
	return known


## VT : sol affiché au centre de l'emprise (clic, anneau de sélection).
func _ground_footprint(i: int) -> void:
	var px := model_px(i)
	_ground_y[i] = terrain.surface_height_at(px.x, px.y)


## VH4 : cercles des villes emblématiques encore sans ville 1:1 (format v2) : le plancher de
## caméra provisoire de ZG4b (`landmark_min_distance`) ne s'applique plus qu'à elles.
func landmark_floor_zones() -> PackedVector3Array:
	var zones := PackedVector3Array()
	for i in _landmarks:
		if landmark_cities != null and landmark_cities.is_enabled() and landmark_cities.has_city(str(data.settlements[i]["id"])):
			continue
		zones.append(_landmarks[i])
	return zones


## Cercles (x, z, rayon) des villes emblématiques, pour le zoom rapproché de la caméra.
func landmark_zones() -> PackedVector3Array:
	return PackedVector3Array(_landmarks.values())


## DC4 (ADR 0082), VT : vrai si un point carte tombe dans l'emprise réelle d'une colonie (hameau
## de `hamlets.json` resté sur une place ajoutée depuis : il n'est pas posé).
func on_settlement_model(px: Vector2) -> bool:
	var index := terrain.chunk_index_at(px.x, px.y)
	var margin := ModelLibrary.HAMLET_SCALE * 0.3
	for dy in [-1, 0, 1]:
		for dx in [-1, 0, 1]:
			for j in _settlements_by_chunk.get(index + dy * terrain.chunks_x + dx, PackedInt32Array()):
				if not _landmarks.has(j) and px.distance_to(model_px(j)) < _model_radius[j] + margin:
					return true
	return false


## RS-K3 : `on_settlement_model` avec les emprises voisines lues une fois par morceau (même test).
func _on_model_disk(px: Vector2) -> bool:
	var index := terrain.chunk_index_at(px.x, px.y)
	var disks: Array = _model_disks.get(index, [])
	if disks.is_empty():
		var centers := PackedVector2Array()
		var radii := PackedFloat64Array()
		var margin := ModelLibrary.HAMLET_SCALE * 0.3
		for dy in [-1, 0, 1]:
			for dx in [-1, 0, 1]:
				for j in _settlements_by_chunk.get(index + dy * terrain.chunks_x + dx, PackedInt32Array()):
					if not _landmarks.has(j):
						centers.append(model_px(j))
						radii.append(_model_radius[j] + margin)
		disks = [centers, radii]
		_model_disks[index] = disks
	var centers: PackedVector2Array = disks[0]
	var radii: PackedFloat64Array = disks[1]
	for k in centers.size():
		if px.distance_to(centers[k]) < radii[k]:
			return true
	return false


## Vrai si un point carte est couvert par une ville emblématique (hameaux).
func covered_by_landmark(px: Vector2) -> bool:
	for zone: Vector3 in _landmarks.values():
		if px.distance_to(Vector2(zone.x, zone.y)) <= zone.z:
			return true
	return false


## RS-K2 : emprises ou ancrages changés : exclusions des hameaux à recalculer.
func _forget_hamlet_exclusions() -> void:
	if data == null:
		return
	_hamlet_keep.resize(data.hamlets.size())
	_hamlet_keep.fill(0)
	_hamlet_tiles.clear()
	_model_disks.clear()


func _build_label(i: int, entry: Dictionary) -> void:
	var label := Label3D.new()
	var kind := str(entry["kind"])
	label.name = "Label_%d" % i
	label.text = str(entry["name"])
	label.font_size = UiType.size(str(LABEL_TYPE.get(kind, UiType.CAPTION)))
	label.font = _label_font(int(LABEL_WEIGHT.get(kind, 500)))
	label.outline_size = LABEL_OUTLINE_PX
	label.modulate = label_color
	label.outline_modulate = _halo(1.0)
	label.billboard = BaseMaterial3D.BILLBOARD_ENABLED
	label.fixed_size = true
	label.pixel_size = 0.0011
	label.no_depth_test = true
	label.render_priority = 4  # DC4 : texte et contour au-dessus des marqueurs (2)
	label.outline_render_priority = 3
	label.visible = false
	var px: Vector2 = entry["px"]
	label.position = Vector3(px.x, map_data.surface_world_at(px.x, px.y), px.y)
	_labels_root.add_child(label)
	_labels.append(label)
	_label_size.append(Vector2.ZERO)
	_label_kind.append(kind)


## Instance du `MultiMesh` d'une colonie : ordre inverse de la priorité, pour que les lieux de
## rang élevé (cités, triées en tête) soient dessinés par-dessus les petits.
func _icon_instance(i: int) -> int:
	return data.settlements.size() - 1 - i


func _build_icons() -> void:
	markers = SettlementMarkers.load_default()
	var count := data.settlements.size()
	_marker_rank.resize(count)
	_marker_size.resize(count)
	_marker_until.resize(count)
	_marker_world.resize(count)
	_marker_holder.resize(count)
	_shielded.resize(count)
	_shielded.fill(0)
	_marker_shown.resize(count)
	_marker_shown.fill(1)
	_marker_screen.resize(count)
	_marker_screen.fill(Vector2(-1.0e6, -1.0e6))
	var quad := QuadMesh.new()
	quad.size = Vector2.ONE
	var multimesh := MultiMesh.new()
	multimesh.transform_format = MultiMesh.TRANSFORM_3D
	multimesh.use_colors = true
	multimesh.use_custom_data = true
	multimesh.mesh = quad
	multimesh.instance_count = count
	for i in count:
		var entry: Dictionary = data.settlements[i]
		var px: Vector2 = entry["px"]
		var kind := str(entry["kind"])
		var rank := markers.rank_of(entry)
		_marker_rank[i] = rank
		_marker_size[i] = markers.size_px(kind, rank) * markers.shield_size_factor()
		_marker_until[i] = markers.visible_until(kind, rank)
		_marker_holder[i] = ""
		var k := _icon_instance(i)
		# DV2 : ancre = position du nom (recalée par `_sync_shield` quand le nom bouge).
		_marker_world[i] = _labels[i].position if i < _labels.size() else Vector3(px.x, map_data.surface_world_at(px.x, px.y), px.y)
		multimesh.set_instance_transform(k, Transform3D(Basis.IDENTITY, _marker_world[i]))
		# r = case d'écu (-1 : aucun) ; DA7d : b = affiché (1) / cède la place (0), a = instant
		# du dernier changement (fondu).
		multimesh.set_instance_color(k, Color(-1.0, 0.0, 1.0, -1.0e4))
		# r = haut du nom au-dessus de l'ancre (px d'étiquette), b = côté de l'écu (px écran).
		multimesh.set_instance_custom_data(k, Color(_shield_lift(i), 0.0, _marker_size[i], _marker_until[i] / 100.0))
	_icon_material = ShaderMaterial.new()
	_icon_material.shader = preload("res://shaders/settlement_icon.gdshader")
	_fade_distance = markers.fade_distance()
	_shield_gap = markers.shield_gap_px()
	_icon_material.set_shader_parameter("gap_px", _shield_gap)
	_icon_material.set_shader_parameter("fade_distance", markers.fade_distance())
	_icon_material.set_shader_parameter("size_scale", icon_size_scale)
	_icon_material.set_shader_parameter("declutter_fade", float(markers.declutter_value("fade_seconds", 0.25)))
	_icon_material.set_shader_parameter("hidden_alpha", float(markers.declutter_value("hidden_alpha", 0.0)))
	declutter_interval = float(markers.declutter_value("interval_seconds", declutter_interval))
	_build_priority_order()
	_icon_material.render_priority = 2
	_icons = MultiMeshInstance3D.new()
	_icons.name = "Icons"
	_icons.multimesh = multimesh
	_icons.material_override = _icon_material
	_icons.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	_icons.extra_cull_margin = 64.0
	add_child(_icons)


## Lot DA3 : écus des détenteurs. L'atlas d'écus est recomposé quand une faction nouvelle
## apparaît (rare : révolte, succession), sinon seules les instances changées sont réécrites.
func _refresh_shields() -> void:
	if _icons == null or markers == null:
		return
	var factions: Array = []
	var missing := heraldry.texture == null
	for entry in data.settlements:
		var controller := str(entry["controller"])
		if controller != "" and not factions.has(controller):
			factions.append(controller)
			if not heraldry.has(controller):
				missing = true
	if missing:
		factions.sort()
		heraldry.build(factions)
		_icon_material.set_shader_parameter("shields", heraldry.texture)
		_icon_material.set_shader_parameter("shield_grid", heraldry.grid())
		_marker_holder.fill("?")
	var multimesh := _icons.multimesh
	for i in data.settlements.size():
		var controller := str(data.settlements[i]["controller"])
		if controller == _marker_holder[i]:
			continue
		_marker_holder[i] = controller
		var k := _icon_instance(i)
		var color := multimesh.get_instance_color(k)
		var cell := heraldry.shield_of(controller)
		_shielded[i] = 1 if cell >= 0 else 0
		color.r = float(cell)
		multimesh.set_instance_color(k, color)


## Côté écran (px) de l'écu d'une colonie (dé-encombrement, picking).
func marker_size(i: int) -> float:
	return _marker_size[i] * icon_size_scale if i >= 0 and i < _marker_size.size() else 24.0


## Vrai si le couple nom + écu de la colonie `i` est affiché à la distance caméra courante (rang
## et dé-encombrement écran DA7d).
func marker_visible(i: int) -> bool:
	return marker_in_tier(i) and _marker_shown[i] == 1


## Vrai si le rang de la colonie `i` affiche son nom et son écu à la distance caméra courante, en
## vue normale (avant dé-encombrement). La colonie sélectionnée reste affichée à toute distance.
func marker_in_tier(i: int) -> bool:
	return i >= 0 and i < _marker_until.size() and _camera_distance < _marker_until_of(i) and _weights.y > 0.01


## Vrai si la colonie `i` a un écu (détenteur armorié).
func has_shield(i: int) -> bool:
	return i >= 0 and i < _shielded.size() and _shielded[i] == 1


## DV2, VT : emprises cliquables et anneau de sélection (vue normale).
func _footprints_on() -> bool:
	return _weights.y > 0.35


func _marker_until_of(i: int) -> float:
	return SELECTED_UNTIL if _is_selected(i) else _marker_until[i]


func _is_selected(i: int) -> bool:
	return i == _selected_index


## Lot DA7d : ordre de priorité fixe des colonies (rang décroissant, puis poids, puis ordre des
## données : cité > ville > château > abbaye > village).
func _build_priority_order() -> void:
	var order: Array = range(data.settlements.size())
	var weights := PackedFloat32Array()
	weights.resize(order.size())
	_max_marker_px = 16.0
	for i in order.size():
		weights[i] = float(data.settlements[i].get("weight", 0))
		_max_marker_px = maxf(_max_marker_px, marker_size(i))
	order.sort_custom(func(a: int, b: int) -> bool:
		if _marker_rank[a] != _marker_rank[b]:
			return _marker_rank[a] > _marker_rank[b]
		if weights[a] != weights[b]:
			return weights[a] > weights[b]
		return a < b)
	_priority_order = PackedInt32Array(order)


func _build_selection_ring() -> void:
	var torus := TorusMesh.new()
	torus.inner_radius = 0.92
	torus.outer_radius = 1.0
	torus.rings = 48
	torus.ring_segments = 6
	var material := StandardMaterial3D.new()
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	material.albedo_color = Color(1.0, 0.84, 0.3)
	material.no_depth_test = true
	material.render_priority = 2
	_selection_ring = MeshInstance3D.new()
	_selection_ring.name = "SelectionRing"
	_selection_ring.mesh = torus
	_selection_ring.material_override = material
	_selection_ring.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	_selection_ring.visible = false
	add_child(_selection_ring)


# --- État de la simulation ---------------------------------------------------------


## Écus des détenteurs et dévastation des provinces. `color_of` (faction → Color) n'est plus lu
## depuis le retrait des bannières de maquettes (VT) ; signature gardée pour les appelants.
func refresh(sim: Object, _color_of: Callable) -> void:
	if data == null:
		return
	data.apply_live(sim)
	if landmark_cities != null and sim != null and sim.has_method("get_date_label"):
		var year := LandmarkModel.year_of(str(sim.call("get_date_label")))
		if year > 0:
			landmark_cities.set_year(year)
	_refresh_shields()
	_refresh_capital(sim)
	var devastation := {}
	if sim != null and (sim.has_method("get_provinces_snapshot") or sim.has_method("get_province_state")):
		var provinces := {}
		for hamlet in data.hamlets:
			provinces[hamlet["province"]] = true
		# PB3d : instantané groupé (partagé avec les autres calques du même rafraîchissement).
		var snapshot := ProvinceSnapshot.of(sim, map_data) if map_data != null else ProvinceSnapshot.read(sim, PackedStringArray(provinces.keys()))
		for province_id in provinces:
			var i := snapshot.index_of(str(province_id))
			devastation[province_id] = float(snapshot.devastation[i]) if i >= 0 else 0.0
	if devastation != _devastation:
		# Seules les tuiles dont un hameau est dans une province à la dévastation changée.
		var changed := {}
		for province_id in devastation:
			if not _devastation.has(province_id) or float(_devastation[province_id]) != float(devastation[province_id]):
				changed[province_id] = true
		for province_id in _devastation:
			if not devastation.has(province_id):
				changed[province_id] = true
		_devastation = devastation
		for index in _hamlet_nodes:
			for h in _hamlets_by_chunk.get(index, []):
				if changed.has(data.hamlets[h]["province"]):
					_hamlet_dirty[index] = true
					break


## Lot DA7d : colonie de la capitale du joueur (la plus prioritaire de la province capitale).
func _refresh_capital(sim: Object) -> void:
	var index := -1
	var facade := get_node_or_null("/root/SimFacade") if is_inside_tree() else null
	if sim != null and facade != null and sim.has_method("get_player_faction"):
		var info: Variant = facade.call("faction_info", str(sim.call("get_player_faction")))
		var province := str((info as Dictionary).get("capital", "")) if info is Dictionary else ""
		if province != "":
			for i in _priority_order:
				if str(data.settlements[i].get("province", "")) == province:
					index = i
					break
	if index != _capital_index:
		_capital_index = index
		_declutter_force = true


## Index de la colonie de la capitale du joueur (-1 si inconnue).
func capital_index() -> int:
	return _capital_index


# --- Mise à jour par image -----------------------------------------------------------


func update_view(camera_distance: float) -> void:
	if data == null:
		return
	_camera_distance = camera_distance
	var tp := Time.get_ticks_usec()  # RS-K : sections `settle/*` du banc `--bench-probe`
	# DV2 (ADR 0124) : détail proche, vue normale, vue stratégique.
	var strategic := tiers.strategic_weight(camera_distance)
	var weights := Vector3(tiers.near_weight(camera_distance), 1.0 - strategic, strategic)
	if weights != _weights:
		_weights = weights
		# DV2 : écus (dé-encombrés par le shader) dans toute la vue normale, estompés au fondu
		# vers le parchemin.
		var icon_alpha := weights.y
		_icon_material.set_shader_parameter("alpha", icon_alpha)
		_icons.visible = icon_alpha > 0.01
		# SZ4 : hameaux à leur taille réelle sous le palier comté, gardés au palier site.
		_hamlets_root.visible = weights.x > 0.35
		_declutter_timer = 0.0
		_declutter_force = true
	if not _labels_placed:
		# VT : étiquettes toujours au sol (plus de maquette) : posées une fois, puis suivies par
		# morceau recalé et par échelle verticale.
		_update_label_heights()
	if MapData.vertical_scale() != _label_scale:
		# ZG4 : toutes les étiquettes suivent l'échelle verticale. RS-K3 (ADR 0051) : tour de
		# réécriture étalé (`LABEL_SLICE` étiquettes par image, reprise au curseur ; un nouveau
		# pas relance un tour complet).
		_label_scale = MapData.vertical_scale()
		_label_left = _labels.size()
	if _labels_dirty:
		_labels_dirty = false
		# SZ6 : seules les colonies des morceaux recalés changent.
		for index: int in _label_chunks:
			for i in _settlements_by_chunk.get(index, PackedInt32Array()):
				_update_label_height(i)
		_label_chunks.clear()
	if _label_left > 0:
		var tv := Time.get_ticks_usec()
		_label_slice(FrameBudget.in_frame())
		PerfProbe.lap("settle/labels/vscale", tv)  # RS-K3
	_update_shield_label_scale()
	if not is_equal_approx(camera_distance, _icon_distance) and _icon_material != null:
		_icon_distance = camera_distance
		_icon_material.set_shader_parameter("camera_distance", camera_distance)
	tp = PerfProbe.lap("settle/labels", tp)
	_update_towns(camera_distance)
	tp = PerfProbe.lap("settle/towns", tp)
	_update_landmark_cities(camera_distance)
	tp = PerfProbe.lap("settle/landmarks", tp)
	if town_far != null:  # VT-E : après les calques 1:1 (masque d'enfoncement à jour)
		town_far.update_view(camera_distance)
	tp = PerfProbe.lap("settle/townfar", tp)
	_update_hamlet_scale(camera_distance)
	var th := PerfProbe.lap("settle/hamlets/scale", tp)  # RS-K2
	_update_hamlets()
	PerfProbe.lap("settle/hamlets/build", th)
	_update_selection_ring()
	tp = PerfProbe.lap("settle/hamlets", tp)
	_declutter_timer -= get_process_delta_time() if is_inside_tree() else 0.0
	if _declutter_timer <= 0.0:
		_declutter_timer = declutter_interval
		# DA7d : recalcul seulement si la caméra a bougé nettement (ou épinglés, palier changés).
		if _declutter_force or _camera_moved():
			_declutter_begin()
	if _dc_pos >= 0:
		_declutter_step(FrameBudget.in_frame())
	_update_declutter_fade()
	PerfProbe.lap("settle/declutter", tp)


func _on_chunk_surface_changed(index: int) -> void:
	for i in _settlements_by_chunk.get(index, PackedInt32Array()):
		_ground_footprint(i)
	if _hamlet_nodes.has(index):
		_hamlet_dirty[index] = true
	# ZG4 : hauteurs des étiquettes une fois par image (et non à chaque morceau recalé).
	_labels_dirty = true
	_label_chunks[index] = true


## SZ6 : morceaux recalés depuis la dernière mise à jour des étiquettes ; VT : étiquettes posées
## au moins une fois ; échelle verticale de la dernière mise à jour complète.
var _label_chunks: Dictionary = {}
var _labels_placed := false
var _label_scale := -1.0
## RS-K3 : tour de réécriture des étiquettes (échelle verticale) : étiquettes par image, curseur
## et entrées restant à voir ; altitude du sol (m, avant affichage) sous chaque étiquette, lue une
## fois (NAN : à lire).
const LABEL_SLICE := 300
var _label_cursor := 0
var _label_left := 0
var _label_ground_m: PackedFloat64Array = PackedFloat64Array()


## Hauteur de toutes les étiquettes (au-dessus du lieu, VT : plus de maquette).
func _update_label_heights() -> void:
	_labels_placed = true
	_label_scale = MapData.vertical_scale()
	_label_chunks.clear()
	_label_left = 0
	for i in _labels.size():
		_update_label_height(i)


## RS-K3 : tranche du tour de réécriture des étiquettes (toutes si `sliced` est faux).
func _label_slice(sliced: bool) -> void:
	var count := _labels.size()
	if count == 0:
		_label_left = 0
		return
	var done := 0
	var i := _label_cursor % count
	while _label_left > 0 and (not sliced or done < LABEL_SLICE):
		_update_label_height(i)
		done += 1
		_label_left -= 1
		i += 1
		if i == count:
			i = 0
	_label_cursor = i


## VT : nom posé au sol du lieu, décalé en pixels écran au-dessus de son emprise réelle
## (`_label_lift_px`). RS-K3 : altitude du sol lue une fois (même valeur que `surface_world_at`),
## écritures seulement si elles changent.
func _update_label_height(i: int) -> void:
	var label := _labels[i]
	var px: Vector2 = data.settlements[i]["px"]
	if _label_ground_m.size() != _labels.size():
		_label_ground_m.resize(_labels.size())
		_label_ground_m.fill(NAN)
	var ground_m := _label_ground_m[i]
	if is_nan(ground_m):
		ground_m = map_data.height_m_at(px.x, px.y)
		_label_ground_m[i] = ground_m
	var label_at := Vector3(px.x, maxf(MapData.display_height(ground_m, px.x, px.y), 0.0) + 0.5, px.y)
	if label.position != label_at:
		label.position = label_at
	_refresh_label_offset(i, _label_lift_px(i))
	_sync_shield(i)


## VT : décalage vertical (px d'étiquette) du nom `i`, réécrit (écu recalé) seulement s'il change
## d'au moins 0,5 px (appelé à chaque passe de dé-encombrement).
func _refresh_label_offset(i: int, lift: float) -> void:
	var label := _labels[i]
	if absf(label.offset.y - lift) >= 0.5 or label.offset.x != 0.0:
		label.offset = Vector2(0.0, lift)
		_sync_shield(i)


## DV2 : écu recalé sur son nom (ancre du `MultiMesh` et haut du texte), écritures seulement si
## elles changent.
func _sync_shield(i: int) -> void:
	if _icons == null or i >= _marker_world.size():
		return
	var at := _labels[i].position
	var k := _icon_instance(i)
	if _marker_world[i] != at:
		_marker_world[i] = at
		_icons.multimesh.set_instance_transform(k, Transform3D(Basis.IDENTITY, at))
	var custom := _icons.multimesh.get_instance_custom_data(k)
	var lift := _shield_lift(i)
	if not is_equal_approx(custom.r, lift):
		custom.r = lift
		_icons.multimesh.set_instance_custom_data(k, custom)


## DV2 : haut du nom `i` au-dessus de son ancre, en px d'étiquette (décalage + demi-texte).
func _shield_lift(i: int) -> float:
	return _labels[i].offset.y + _label_text_size(i).y * 0.5


## DV2 : échelle écran des étiquettes transmise au shader des écus quand elle change.
func _update_shield_label_scale() -> void:
	if _icon_material == null or not is_inside_tree():
		return
	var camera := get_viewport().get_camera_3d()
	if camera == null:
		return
	var scale := _label_screen_scale(camera)
	if not is_equal_approx(scale, _shield_label_scale):
		_shield_label_scale = scale
		_icon_material.set_shader_parameter("label_scale", scale)


## DV2 : rectangle écran de l'écu `i`, l'ancre projetée en `anchor` (échelle des étiquettes
## `scale`), élargi de `margin` px.
func _shield_rect(i: int, anchor: Vector2, scale: float, margin: float) -> Rect2:
	var size := marker_size(i)
	var bottom := anchor.y - _shield_lift(i) * scale - _shield_gap
	return Rect2(Vector2(anchor.x - size * 0.5, bottom - size), Vector2(size, size)).grow(margin)


## DV2 : opacité du nom `i` : vue normale, fondu de retrait par rang (comme l'écu, cf. shader).
## La densité des noms suit la seule règle par rang (`SettlementMarkers.visible_until`).
func _label_alpha(i: int) -> float:
	var until := _marker_until_of(i)
	var fade := _fade_distance
	return clampf(_weights.y * (1.0 - smoothstep(until - fade, until, _camera_distance)), 0.0, 1.0)


## Lot DA7d (ADR 0066) : dé-encombrement écran unique des marqueurs et des noms. Colonie par
## colonie dans l'ordre de priorité (épinglés, puis rang, poids, ordre des données) : le marqueur,
## puis son nom. Un marqueur qui recouvre un rectangle déjà posé cède la place (fondu du shader)
## et son nom avec lui ; un nom qui recouvre un rectangle posé est masqué. Rectangles des noms
## mesurés avec la police (DC4, `_label_screen_rect`) ; grille spatiale (`MarkerDeclutter`),
## coût linéaire avec ~1 200 colonies. Remplace le masquage des seuls noms de DC4.
func declutter() -> void:
	if _declutter_begin():
		_declutter_step(false)


## RS-K3 (ADR 0051) : passe de dé-encombrement en cours, étalée sur les images (`DECLUTTER_SLICE`
## colonies par image dans une image ouverte). Tout est mesuré avec l'état de caméra figé au début
## de la passe (projection recalculée ici, même formule que `Camera3D.unproject_position`) : les
## tranches restent cohérentes entre elles quand la caméra bouge.
const DECLUTTER_SLICE := 600
var _dc_sequence := PackedInt32Array()
var _dc_pos := -1
var _dc_pins := PackedInt32Array()
var _dc_usec := 0
var _dc_view := Transform3D()
var _dc_projection := Projection()
var _dc_size := Vector2.ONE
var _dc_origin := Vector3.ZERO
var _dc_eye := Vector3.FORWARD
var _dc_near := 0.05
var _dc_label_screen := Rect2()
var _dc_marker_margin := 2.0
var _dc_label_margin := 4.0
var _dc_icons_on := false
var _dc_close_w := 0.0
var _dc_label_range := INF
var _dc_scale := 1.0
var _dc_focal := 0.0


## RS-K3 : commence une passe (état de caméra figé, obstacles et épinglés posés) ; faux sans caméra.
func _declutter_begin() -> bool:
	var camera := get_viewport().get_camera_3d() if is_inside_tree() else null
	if camera == null or data == null or _priority_order.is_empty():
		return false
	var t0 := Time.get_ticks_usec()
	var tp := t0  # RS-K3 : sous-sections du banc `--bench-probe`
	_declutter_force = false
	_declutter_camera = _camera_state(camera)
	var screen := get_viewport().get_visible_rect()
	_dc_size = screen.size
	_dc_view = camera.get_camera_transform().affine_inverse()
	_dc_projection = camera.get_camera_projection()
	var eye_transform := camera.global_transform
	_dc_origin = eye_transform.origin
	_dc_eye = -eye_transform.basis.z.normalized()
	_dc_near = camera.near
	var spill := float(markers.declutter_value("label_screen_margin", 0.2))
	_dc_label_screen = screen.grow_individual(screen.size.x * spill, screen.size.y * spill, screen.size.x * spill, screen.size.y * spill)
	_dc_marker_margin = float(markers.declutter_value("marker_margin_px", 2.0))
	_dc_label_margin = float(markers.declutter_value("label_margin_px", declutter_margin))
	_dc_icons_on = _icons != null and _icons.visible
	_placer.reset(_max_marker_px * icon_size_scale + _dc_marker_margin * 2.0)
	# CV3-0 (#7) : les plaques/étendards d'armée sont réservés avant les colonies (déjà placés,
	# stables d'une frame à l'autre) ; un marqueur ou un nom de colonie qui les recouvre cède
	# la place, au lieu de se superposer (étiquettes qui se chevauchaient au pied de l'armée).
	if label_obstacles.is_valid():
		for rect: Rect2 in label_obstacles.call(camera):
			_placer.try_place(rect, -2, true)
	_dc_pins = _pinned_indices()
	# ZG4 : paliers vallée / site (vue rasante) : seulement les colonies proches, l'horizon ne se
	# couvre pas de noms.
	_dc_close_w = tiers.valley_weight(_camera_distance) if tiers != null else 0.0
	_dc_label_range = tiers.close_label_range_factor * _camera_distance if tiers != null else INF
	_dc_scale = _label_screen_scale(camera)
	_dc_focal = _focal_px(camera)
	_dc_sequence = PackedInt32Array(_dc_pins)
	for i in _priority_order:
		if not _dc_pins.has(i):
			_dc_sequence.append(i)
	_dc_pos = 0
	PerfProbe.lap("settle/declutter/prep", tp)
	_dc_usec = Time.get_ticks_usec() - t0
	return true


## RS-K3 : point écran de `p` avec la caméra figée de la passe (`Camera3D.unproject_position`).
func _dc_project(p: Vector3) -> Vector2:
	var local := _dc_view * p
	var clip := _dc_projection * Vector4(local.x, local.y, local.z, 1.0)
	return Vector2((clip.x / clip.w * 0.5 + 0.5) * _dc_size.x, (-clip.y / clip.w * 0.5 + 0.5) * _dc_size.y)


## RS-K3 : `Camera3D.is_position_behind` avec la caméra figée de la passe.
func _dc_behind(p: Vector3) -> bool:
	return _dc_eye.dot(p - _dc_origin) < _dc_near


## RS-K3 : tranche de la passe en cours (toute la passe si `sliced` est faux).
func _declutter_step(sliced: bool) -> void:
	var t0 := Time.get_ticks_usec()
	var end := _dc_sequence.size() if not sliced else mini(_dc_sequence.size(), _dc_pos + DECLUTTER_SLICE)
	for n in range(_dc_pos, end):
		var i := _dc_sequence[n]
		var pinned := n < _dc_pins.size()
		_marker_screen[i] = Vector2(-1.0e6, -1.0e6)
		# DV2 : nom et écu forment un seul couple, affiché ou cédé ensemble. Tests du moins cher
		# au plus cher (hors palier : le shader replie l'écu lui-même).
		var alpha := _label_alpha(i) if _dc_icons_on and marker_in_tier(i) else 0.0
		if alpha < 0.02:
			_set_marker_shown(i, true)
			_show_label(i, false, alpha)
			continue
		var label := _labels[i]
		var at := label.global_position
		if _dc_behind(at) or (_dc_close_w > 0.5 and _dc_origin.distance_to(at) > _dc_label_range):
			_set_marker_shown(i, true)
			_show_label(i, false, alpha)
			continue
		# VT : nom décalé au-dessus de l'emprise projetée (suit le zoom à chaque passe).
		_refresh_label_offset(i, _label_lift_with(i, _dc_origin, _dc_focal, _dc_scale))
		var anchor := _dc_project(at)
		# Rectangle du nom (cf. `_label_screen_rect`) et de l'écu (cf. `_shield_rect`).
		var text := _label_text_size(i) * _dc_scale
		var label_center := anchor - label.offset * Vector2(-1.0, 1.0) * _dc_scale
		var rect := Rect2(label_center - text * 0.5, text).grow(_dc_label_margin)
		var shielded := _shielded[i] == 1
		var shield_rect := _shield_rect(i, anchor, _dc_scale, _dc_marker_margin) if shielded else Rect2()
		var shown := false
		if _dc_label_screen.intersects(rect) or (shielded and _dc_label_screen.intersects(shield_rect)):
			shown = pinned or not (_placer.overlaps(rect, i) or (shielded and _placer.overlaps(shield_rect, i)))
			if shown:
				_placer.try_place(rect, i, true)
				if shielded:
					_placer.try_place(shield_rect, i, true)
					_marker_screen[i] = shield_rect.get_center()
		_set_marker_shown(i, shown)
		_show_label(i, shown, alpha)
	_dc_pos = end
	_dc_usec += Time.get_ticks_usec() - t0
	if _dc_pos >= _dc_sequence.size():
		_dc_pos = -1
		last_declutter_ms = float(_dc_usec) / 1000.0
	PerfProbe.lap("settle/declutter/run", t0)


## Affiche ou masque l'étiquette `i` ; couleurs réécrites seulement quand l'opacité change.
func _show_label(i: int, shown: bool, alpha: float) -> void:
	var label := _labels[i]
	if label.visible != shown:
		label.visible = shown
	if not shown or is_equal_approx(label.modulate.a, alpha):
		return
	var modulate := label_color
	modulate.a = alpha
	label.modulate = modulate
	label.outline_modulate = _halo(alpha)


## PO3 : couleur du halo de parchemin pour une opacité d'étiquette `alpha`.
func _halo(alpha: float) -> Color:
	var outline := label_outline
	outline.a = alpha * LABEL_HALO_ALPHA
	return outline


static var _label_fonts: Dictionary = {}


## PO3 : EB Garamond à la graisse `weight` (axe variable `wght`), partagée ; null si la police manque.
static func _label_font(weight: int) -> Font:
	if _label_fonts.has(weight):
		return _label_fonts[weight]
	var font: Font = null
	if ResourceLoader.exists(LABEL_FONT_PATH):
		var variation := FontVariation.new()
		variation.base_font = load(LABEL_FONT_PATH) as Font
		variation.variation_opentype = {TextServerManager.get_primary_interface().name_to_tag("wght"): weight}
		font = variation
	_label_fonts[weight] = font
	return font


## DV2, VT : décalage du nom `i` au-dessus du lieu (px du `Label3D`) : rayon écran de son emprise
## réelle (plafonné à `LABEL_FOOTPRINT_MAX_PX`) + `LABEL_GAP_PX`, ramenés à l'échelle écran, +
## demi-hauteur du texte mesuré (DC4). L'écu est posé au-dessus du nom (`_shield_lift`).
func _label_lift_px(i: int) -> float:
	# RS-K2 : caméra lue une fois par image (2 147 étiquettes par recalcul).
	var frame := Engine.get_process_frames()
	if frame != _lift_frame:
		_lift_frame = frame
		var camera := get_viewport().get_camera_3d() if is_inside_tree() else null
		_lift_scale = _label_screen_scale(camera) if camera != null else 1.0
		_lift_focal = _focal_px(camera)
		_lift_eye = camera.global_position if camera != null else Vector3.ZERO
	return _label_lift_with(i, _lift_eye, _lift_focal, _lift_scale)


## VT : `_label_lift_px` pour une caméra donnée (œil, focale en px, échelle des étiquettes).
func _label_lift_with(i: int, eye: Vector3, focal: float, scale: float) -> float:
	var footprint_px := 0.0
	if focal > 0.0 and i < _model_radius.size():
		var distance := maxf(eye.distance_to(_labels[i].position), 1e-3)
		footprint_px = minf(_model_radius[i] * focal / distance, LABEL_FOOTPRINT_MAX_PX)
	return (LABEL_GAP_PX + footprint_px) / maxf(scale, 0.1) + _label_text_size(i).y * 0.5


## VT : focale (px écran par unité monde à distance 1) d'une caméra en perspective, 0 sinon.
func _focal_px(camera: Camera3D) -> float:
	if camera == null or camera.projection != Camera3D.PROJECTION_PERSPECTIVE:
		return 0.0
	return camera.get_viewport().get_visible_rect().size.y / (2.0 * tan(deg_to_rad(camera.fov) * 0.5))


## DC4 : taille du texte du nom `i` (police, contour compris), mesurée une fois.
func _label_text_size(i: int) -> Vector2:
	var size := _label_size[i]
	if size == Vector2.ZERO:
		var label := _labels[i]
		var font := label.font if label.font != null else ThemeDB.fallback_font
		size = font.get_string_size(label.text, HORIZONTAL_ALIGNMENT_LEFT, -1, label.font_size) + Vector2.ONE * float(label.outline_size)
		_label_size[i] = size
	return size


## DC4 : échelle écran d'un `Label3D` à taille fixe (cf. `LabelPlacer.label3d_screen_rect`).
func _label_screen_scale(camera: Camera3D) -> float:
	if camera.projection != Camera3D.PROJECTION_PERSPECTIVE or _labels.is_empty():
		return 1.0
	return _labels[0].pixel_size * camera.get_viewport().get_visible_rect().size.y / (2.0 * tan(deg_to_rad(camera.fov) * 0.5))


## DC4 : rectangle écran du nom `i`, texte mesuré avec la police (contour compris, mis en cache),
## élargi de `margin` px.
func _label_screen_rect(i: int, camera: Camera3D, scale: float, margin: float) -> Rect2:
	var label := _labels[i]
	var size := _label_text_size(i) * scale
	var center := camera.unproject_position(label.global_position) - label.offset * Vector2(-1.0, 1.0) * scale
	return Rect2(center - size * 0.5, size).grow(margin)


## Lot DA7d : colonies toujours affichées : sélection, survol, capitale du joueur (dans cet ordre).
func _pinned_indices() -> PackedInt32Array:
	var pins := PackedInt32Array()
	if bool(markers.declutter_value("pin_selected", true)) and selected_id != "":
		pins.append(int(data.index_by_id.get(selected_id, -1)))
	_hovered_index = -1
	if bool(markers.declutter_value("pin_hovered", true)) and _weights.y > 0.01 and is_inside_tree():
		# Survol : le marqueur affiché sous la souris (centres écran du recalcul précédent) reste
		# affiché pendant le zoom à la molette.
		var mouse := get_viewport().get_mouse_position()
		var best := INF
		for i in _marker_screen.size():
			var d := _marker_screen[i].distance_squared_to(mouse)
			var radius := marker_size(i) * PICK_ICON_FRACTION
			if d < radius * radius and d < best:
				best = d
				_hovered_index = i
		if _hovered_index >= 0 and not pins.has(_hovered_index):
			pins.append(_hovered_index)
	if bool(markers.declutter_value("pin_player_capital", true)) and _capital_index >= 0 and not pins.has(_capital_index):
		pins.append(_capital_index)
	var result := PackedInt32Array()
	for i in pins:
		if i >= 0 and i < data.settlements.size():
			result.append(i)
	return result


## Lot DA7d : bascule l'état affiché / cédé d'un marqueur (fondu côté shader, sans CPU par image).
func _set_marker_shown(i: int, shown: bool) -> void:
	var value := 1 if shown else 0
	if _marker_shown[i] == value:
		return
	_marker_shown[i] = value
	if _icons == null:
		return
	var k := _icon_instance(i)
	var color := _icons.multimesh.get_instance_color(k)
	color.b = float(value)
	color.a = _declutter_clock()
	_icons.multimesh.set_instance_color(k, color)
	_declutter_fade_until = _declutter_clock() + float(markers.declutter_value("fade_seconds", 0.25)) + 0.05


## Horloge du fondu (s), ramenée sous 10 h pour garder la précision d'un flottant 32 bits.
static func _declutter_clock() -> float:
	return float(Time.get_ticks_msec() % 36000000) / 1000.0


## Horloge du fondu transmise au shader tant qu'un fondu est en cours (puis une dernière fois).
func _update_declutter_fade() -> void:
	if _declutter_fade_until < 0.0 or _icon_material == null:
		return
	var now := _declutter_clock()
	_icon_material.set_shader_parameter("declutter_now", now)
	if now > _declutter_fade_until:
		_declutter_fade_until = -1.0


## État de caméra comparé entre deux recalculs : position, axe de visée, distance, écran.
func _camera_state(camera: Camera3D) -> Array:
	return [camera.global_position, -camera.global_transform.basis.z, _camera_distance, get_viewport().get_visible_rect().size]


## Vrai si la caméra a bougé nettement depuis le dernier recalcul (seuils des données).
func _camera_moved() -> bool:
	var camera := get_viewport().get_camera_3d() if is_inside_tree() else null
	if camera == null or markers == null:
		return false
	if _declutter_camera.size() < 4:
		return true
	var previous_distance: float = _declutter_camera[2]
	var distance_ratio := absf(_camera_distance - previous_distance) / maxf(previous_distance, 1e-3)
	if distance_ratio > float(markers.declutter_value("recompute_distance_ratio", 0.015)):
		return true
	var moved := camera.global_position.distance_to(_declutter_camera[0])
	if moved > float(markers.declutter_value("recompute_move_fraction", 0.01)) * maxf(_camera_distance, 1e-3):
		return true
	var axis: Vector3 = _declutter_camera[1]
	if rad_to_deg(axis.angle_to(-camera.global_transform.basis.z)) > float(markers.declutter_value("recompute_turn_degrees", 0.5)):
		return true
	return get_viewport().get_visible_rect().size != _declutter_camera[3]




## Lot UX1 : rectangles écran des noms de colonies affichés (obstacles des plaques d'armée).
func screen_label_rects(camera: Camera3D) -> Array[Rect2]:
	var rects: Array[Rect2] = []
	if camera == null:
		return rects
	var view_height := camera.get_viewport().get_visible_rect().size.y
	for label in _labels:
		if label.visible and label.modulate.a > 0.15 and not camera.is_position_behind(label.global_position):
			rects.append(LabelPlacer.label3d_screen_rect(label, camera, view_height))
	return rects


## Lot DA7d, DV2 : rectangle écran de l'écu `i` (marge `margin` en px).
func _marker_rect(i: int, camera: Camera3D, margin: float) -> Rect2:
	return _shield_rect(i, camera.unproject_position(_marker_world[i]), _label_screen_scale(camera), margin)


## Lot DA7d (mesures, tests) : rectangles écran (sans marge) des marqueurs et des noms affichés
## dans l'écran : {rects, owners (index de colonie), markers, labels}.
## `kinds` (parallèle à `rects`/`owners`) : "marker" ou "label", pour filtrer par type (CV3-0 #7 :
## un marqueur de capitale épinglé peut toucher une plaque d'armée, un nom de colonie non).
func screen_occupancy(camera: Camera3D) -> Dictionary:
	var rects: Array[Rect2] = []
	var owners := PackedInt32Array()
	var kinds: Array[String] = []
	var marker_count := 0
	var label_count := 0
	if camera == null or data == null:
		return {"rects": rects, "owners": owners, "kinds": kinds, "markers": 0, "labels": 0}
	var screen := camera.get_viewport().get_visible_rect()
	var icons_on := _icons != null and _icons.visible
	for i in data.settlements.size():
		if icons_on and marker_visible(i) and has_shield(i):
			if not camera.is_position_behind(_marker_world[i]):
				var rect := _marker_rect(i, camera, 0.0)
				if screen.intersects(rect):
					rects.append(rect)
					owners.append(i)
					kinds.append("marker")
					marker_count += 1
		var label := _labels[i]
		if label.visible and label.modulate.a > 0.02 and not camera.is_position_behind(label.global_position):
			var lrect := _label_screen_rect(i, camera, _label_screen_scale(camera), 0.0)
			if screen.intersects(lrect):
				rects.append(lrect)
				owners.append(i)
				kinds.append("label")
				label_count += 1
	return {"rects": rects, "owners": owners, "kinds": kinds, "markers": marker_count, "labels": label_count}


func visible_label_count() -> int:
	var count := 0
	for label in _labels:
		if label.visible:
			count += 1
	return count


# --- Hameaux -------------------------------------------------------------------------


## SZ4 : nouvelle échelle des hameaux (par pas de `rewrite_step`) → transformations des tuiles
## construites réécrites depuis les données gardées à la construction (sans relire le relief).
func _update_hamlet_scale(camera_distance: float) -> void:
	var props := MapPropScale.shared()
	var wanted := props.hamlet_scale(camera_distance)
	if not props.needs_rewrite(_hamlet_scale, wanted):
		return
	_hamlet_scale = wanted
	for node: Node3D in _hamlet_nodes.values():
		for mmi in node.get_children():
			var instance := mmi as MultiMeshInstance3D
			if instance != null and instance.multimesh != null and instance.has_meta("hamlet_entries"):
				_write_hamlet_transforms(instance.multimesh, instance.get_meta("hamlet_entries"))


## SZ4 : transformations des hameaux à l'échelle courante. Pose : entre le sol au centre (taille
## réelle, emprise de quelques dizaines de mètres) et le point le plus bas de l'emprise de carte
## (taille de carte, rien ne flotte sur une pente), au prorata de l'échelle.
func _write_hamlet_transforms(multimesh: MultiMesh, entries: Array) -> void:
	multimesh.buffer = hamlet_buffer(entries, _hamlet_scale)


## PB3g : tampon `MultiMesh.buffer` des hameaux, écrit en une fois (disposition de
## `set_instance_transform` : chaque ligne de la base suivie de la composante de l'origine) au
## lieu d'un appel au serveur de rendu par instance.
static func hamlet_buffer(entries: Array, s: float) -> PackedFloat32Array:
	var buffer := PackedFloat32Array()
	buffer.resize(entries.size() * 12)
	for t in entries.size():
		var e: PackedFloat32Array = entries[t]
		var basis := Basis(Vector3.UP, e[2]).scaled(Vector3.ONE * (e[3] * s))
		var y := lerpf(e[4], e[5], s) - 0.03 * s
		var o := t * 12
		buffer[o] = basis.x.x
		buffer[o + 1] = basis.y.x
		buffer[o + 2] = basis.z.x
		buffer[o + 3] = e[0]
		buffer[o + 4] = basis.x.y
		buffer[o + 5] = basis.y.y
		buffer[o + 6] = basis.z.y
		buffer[o + 7] = y
		buffer[o + 8] = basis.x.z
		buffer[o + 9] = basis.y.z
		buffer[o + 10] = basis.z.z
		buffer[o + 11] = e[1]
	return buffer

func _update_hamlets() -> void:
	var show := _weights.x > 0.35
	var builds := 0
	for index in _hamlets_by_chunk:
		var level := terrain.chunk_level(index)
		var node: Node3D = _hamlet_nodes.get(index)
		if show and level >= 1:
			if node == null or _hamlet_dirty.has(index):
				if builds >= max_hamlet_builds_per_frame or (builds > 0 and not FrameBudget.has_time()):
					continue
				builds += 1
				_build_hamlets(index)
		elif node != null and level == 0:
			node.queue_free()
			_hamlet_nodes.erase(index)
			_hamlet_dirty.erase(index)
	stats["hamlet_chunks"] = _hamlet_nodes.size()


## Construit tout de suite hameaux et villes 1:1 voulus (captures, tests).
func flush() -> void:
	var saved := max_hamlet_builds_per_frame
	max_hamlet_builds_per_frame = 1 << 20
	FrameBudget.unlimited = true
	_update_hamlets()
	FrameBudget.unlimited = false
	max_hamlet_builds_per_frame = saved
	if towns != null:  # ZG6 : villes 1:1 autour de la caméra
		towns.flush()
		_update_towns(_camera_distance)
	if landmark_cities != null:  # VH4 : villes emblématiques 1:1
		landmark_cities.flush()
		_update_landmark_cities(_camera_distance)
	if town_far != null:  # VT-E : lointain des villes
		town_far.flush()
		town_far.update_view(_camera_distance)
	_labels_dirty = false
	_update_label_heights()


func hamlet_instance_count() -> int:
	var total := 0
	for node in _hamlet_nodes.values():
		for mmi in (node as Node3D).get_children():
			total += (mmi as MultiMeshInstance3D).multimesh.instance_count
	return total


## RS-K2 : tables mémorisées des hameaux à la taille des données.
func _hamlet_memo_ready() -> bool:
	return _hamlet_keep.size() == data.hamlets.size() and _hamlet_seed.size() == data.hamlets.size()


## RS-K3 : hameaux posés du morceau `index` (exclusions et graines mémorisées au passage) et
## points de leurs hauteurs : [hameaux, points (5 par hameau), lacets, échelles].
func _hamlet_tile(index: int) -> Array:
	var kept := PackedInt32Array()
	var points := PackedVector2Array()
	var yaws := PackedFloat32Array()
	var scales := PackedFloat32Array()
	var memo := _hamlet_memo_ready()
	for h in _hamlets_by_chunk[index]:
		var hamlet: Dictionary = data.hamlets[h]
		var px: Vector2 = hamlet["px"]
		var keep := _hamlet_keep[h] if memo else 0
		if keep == 2:
			continue
		if keep == 0 and not _landmarks.is_empty() and covered_by_landmark(px):
			if memo:
				_hamlet_keep[h] = 2
			continue
		var seed_value: int = _hamlet_seed[h] if memo else -1
		if seed_value < 0:
			seed_value = _hamlet_seed_of(h)
			if memo:
				_hamlet_seed[h] = seed_value
		px = hamlet_px(h)  # ZG5b : ancrage fin (tirages inchangés)
		if keep == 0:
			if _on_model_disk(px):
				if memo:
					_hamlet_keep[h] = 2
				continue
			if memo:
				_hamlet_keep[h] = 1
		var yaw := float((seed_value / 13) % 628) / 100.0
		var scale := ModelLibrary.HAMLET_SCALE * (0.85 + float((seed_value / 17) % 30) / 100.0)
		kept.append(h)
		yaws.append(yaw)
		scales.append(scale)
		points.append(px)
		for k in 4:
			var angle := k * TAU / 4.0 + yaw
			points.append(Vector2(px.x + cos(angle) * scale * 0.4, px.y + sin(angle) * scale * 0.4))
	return [kept, points, yaws, scales]


## Graine de tirage du hameau `h` (variante, lacet, taille, incendie).
func _hamlet_seed_of(h: int) -> int:
	var hamlet: Dictionary = data.hamlets[h]
	return _hash(str(hamlet["name"]) + str(hamlet["px"]))


func _build_hamlets(index: int) -> void:
	var tp := Time.get_ticks_usec()  # RS-K3 : sous-sections du banc `--bench-probe`
	_hamlet_dirty.erase(index)
	var previous: Node3D = _hamlet_nodes.get(index)
	if previous != null:
		previous.queue_free()
	var node := Node3D.new()
	node.name = "Hamlets_%d" % index
	_hamlets_root.add_child(node)
	_hamlet_nodes[index] = node
	var meshes := ModelLibrary.hamlet_meshes()
	if meshes.is_empty():
		return
	# Groupes : (variante, brûlé) → transformations.
	var groups := {}
	var tile: Array = _hamlet_tiles.get(index, [])
	if tile.is_empty():
		tile = _hamlet_tile(index)
		if _hamlet_memo_ready():
			_hamlet_tiles[index] = tile
	var kept: PackedInt32Array = tile[0]
	var points: PackedVector2Array = tile[1]
	var yaws: PackedFloat32Array = tile[2]
	var scales: PackedFloat32Array = tile[3]
	var memo := _hamlet_memo_ready()
	tp = PerfProbe.lap("settle/hamlets/prep", tp)
	var heights := terrain.surface_heights_at(points)
	tp = PerfProbe.lap("settle/hamlets/heights", tp)
	for p in kept.size():
		var h := kept[p]
		var seed_value := _hamlet_seed[h] if memo else _hamlet_seed_of(h)
		var devastation: float = _devastation.get(data.hamlets[h]["province"], 0.0)
		var burned := devastation >= BURN_THRESHOLD and float((seed_value / 7) % 100) < devastation
		var key := (seed_value % meshes.size()) * 2 + (1 if burned else 0)
		var px := points[p * 5]
		var center := heights[p * 5]
		var low := center
		for k in range(1, 5):
			low = minf(low, heights[p * 5 + k])
		if not groups.has(key):
			groups[key] = []
		# SZ4 : (x, z, lacet, échelle de carte, sol au centre, sol le plus bas de l'emprise de carte).
		groups[key].append(PackedFloat32Array([px.x, px.y, yaws[p], scales[p], center, low]))
	for key in groups:
		var entries: Array = groups[key]
		var multimesh := MultiMesh.new()
		multimesh.transform_format = MultiMesh.TRANSFORM_3D
		multimesh.mesh = meshes[key / 2]
		multimesh.instance_count = entries.size()
		_write_hamlet_transforms(multimesh, entries)
		var mmi := MultiMeshInstance3D.new()
		mmi.set_meta("hamlet_entries", entries)
		mmi.multimesh = multimesh
		if key % 2 == 1:
			mmi.material_override = _burned_material
		mmi.visibility_range_end = tiers.hamlet_range + terrain.chunk_px
		node.add_child(mmi)
	PerfProbe.lap("settle/hamlets/mesh", tp)


# --- Picking et sélection ------------------------------------------------------------


## Colonie sous un point écran (emprise réelle, écu ou nom en vue normale), "" sinon.
func pick_screen(screen_position: Vector2) -> String:
	return str(pick_screen_scored(screen_position).get("id", ""))


## Q2 : comme `pick_screen`, avec `score` (distance au centre / rayon de prise, 0 = en plein
## centre) pour départager une colonie et une armée sous le même clic ; {} si rien.
func pick_screen_scored(screen_position: Vector2) -> Dictionary:
	var camera := get_viewport().get_camera_3d() if is_inside_tree() else null
	if camera == null or data == null:
		return {}
	var near := _footprints_on()
	var icons := _weights.y > 0.01
	var scale := _label_screen_scale(camera)
	var best := ""
	var best_score := INF
	var right := camera.global_transform.basis.x
	var eye := camera.global_position
	var model_range_sq := tiers.model_range * tiers.model_range
	var focal := _focal_px(camera)
	for i in data.settlements.size():
		var score := INF
		if near:
			score = _model_pick_score(i, camera, right, eye, model_range_sq, screen_position, focal)
		if icons and marker_visible(i):  # DA7d : pas les couples cédés
			# DV2 : l'écu et le nom se cliquent comme l'emprise (vue normale entière).
			score = minf(score, _pair_pick_score(i, camera, scale, screen_position))
		if score < best_score:
			best_score = score
			best = str(data.settlements[i]["id"])
	return {"id": best, "score": best_score} if best != "" else {}


## VT : score de picking (distance au centre / rayon de prise, INF hors prise) de l'emprise réelle
## de la colonie `i` (rayon écran d'au moins `PICK_MIN_PX`), dans la portée `model_range`.
func _model_pick_score(i: int, camera: Camera3D, right: Vector3, eye: Vector3, model_range_sq: float, screen_position: Vector2, focal: float) -> float:
	var px := model_px(i)
	var center := Vector3(px.x, _ground_y[i], px.y)
	if eye.distance_squared_to(center) > model_range_sq or camera.is_position_behind(center):
		return INF
	var screen_center := camera.unproject_position(center)
	var d := screen_center.distance_to(screen_position)
	# Rejet rapide avant la projection du bord (2 147 colonies à chaque clic).
	if focal > 0.0 and d > maxf(_model_radius[i] * focal / maxf(eye.distance_to(center) - _model_radius[i], 1e-3), PICK_MIN_PX):
		return INF
	var edge := camera.unproject_position(center + right * _model_radius[i])
	var radius_px := maxf(screen_center.distance_to(edge), PICK_MIN_PX)
	return d / radius_px if d < radius_px else INF


## DV2 : score de picking du couple nom + écu `i` (écu : disque de `PICK_ICON_FRACTION` × sa
## taille ; nom : son rectangle, score ≥ 0,5 pour laisser la priorité à un écu ou une maquette
## visés en plein).
func _pair_pick_score(i: int, camera: Camera3D, scale: float, screen_position: Vector2) -> float:
	var world := _marker_world[i]
	if camera.is_position_behind(world):
		return INF
	var anchor := camera.unproject_position(world)
	var score := INF
	if has_shield(i):
		var radius := marker_size(i) * PICK_ICON_FRACTION
		var d_icon := _shield_rect(i, anchor, scale, 0.0).get_center().distance_to(screen_position)
		if d_icon < radius:
			score = d_icon / radius
	if _labels[i].visible:
		var rect := _label_screen_rect(i, camera, scale, 2.0)
		if rect.has_point(screen_position):
			var half := maxf(rect.size.length() * 0.5, 1.0)
			score = minf(score, 0.5 + 0.5 * rect.get_center().distance_to(screen_position) / half)
	return score


## Sélectionne une colonie ("" = aucune) : surbrillance de l'écu, anneau au sol, signal.
func select(id: String) -> void:
	if _icons != null and selected_id != "" and data.index_by_id.has(selected_id):
		var old := _icon_instance(data.index_by_id[selected_id])
		var custom := _icons.multimesh.get_instance_custom_data(old)
		custom.g = 0.0
		custom.a = _marker_until[data.index_by_id[selected_id]] / 100.0
		_icons.multimesh.set_instance_custom_data(old, custom)
	selected_id = id if data.index_by_id.has(id) else ""
	_selected_index = int(data.index_by_id.get(selected_id, -1))
	_declutter_force = true  # DA7d : la sélection est épinglée
	if selected_id != "":
		var index: int = data.index_by_id[selected_id]
		var custom_new := _icons.multimesh.get_instance_custom_data(_icon_instance(index))
		custom_new.g = 1.0
		custom_new.a = SELECTED_UNTIL / 100.0  # DA7d : la sélection reste affichée à toute distance
		_icons.multimesh.set_instance_custom_data(_icon_instance(index), custom_new)
		var entry: Dictionary = data.settlements[index]
		print("SettlementLayer: selected %s (%s, %s, controller %s)" % [selected_id, entry["name"], entry["kind"], entry["controller"]])
		settlement_selected.emit(selected_id)
	_update_selection_ring()


func _update_selection_ring() -> void:
	if _selection_ring == null:
		return
	var index: int = data.index_by_id.get(selected_id, -1) if data != null else -1
	var show := index >= 0 and _footprints_on()
	# Q8 : dessiné sans test de profondeur, l'anneau vu de plus près que sa taille devenait un
	# disque jaune plein, puis un arc en travers du ciel au zoom minimal : masqué au sol.
	var radius := _model_radius[index] * 1.1 if index >= 0 else 0.0
	show = show and _camera_distance > radius * RING_HIDE_DISTANCE_FACTOR
	_selection_ring.visible = show
	if show:
		# VT : anneau autour de l'emprise réelle.
		var px := model_px(index)
		_selection_ring.position = Vector3(px.x, _ground_y[index] + 0.15, px.y)
		_selection_ring.scale = Vector3(radius, 1.0, radius)


## Position monde d'une colonie (maquette posée, sinon relief), Vector3.ZERO si inconnue.
func world_position_of(id: String) -> Vector3:
	var index: int = data.index_by_id.get(id, -1) if data != null else -1
	if index < 0:
		return Vector3.ZERO
	var px: Vector2 = data.settlements[index]["px"]
	return Vector3(px.x, terrain.surface_height_at(px.x, px.y), px.y)


## Lot ZG5b : position de rendu de la maquette `i` (ancrage fin, sinon position de règle).
func model_px(i: int) -> Vector2:
	return _anchor_px.get(i, data.settlements[i]["px"])


## Lot ZG5b : position de rendu du hameau `h` (ancrage fin, sinon `hamlets.json`).
func hamlet_px(h: int) -> Vector2:
	if h < _hamlet_anchors.size():
		var a := _hamlet_anchors[h]
		return Vector2(a.x, a.y)
	return data.hamlets[h]["px"]


## Lot ZG5b : emprises des colonies (hors villes emblématiques) et hameaux posés aux ancrages fins
## de `fine_anchors.json` (déplacés de ≤ 300 m hors des lits et des pentes fortes), puis recalés
## sur la surface affichée. Icônes, étiquettes, et règles gardent les positions de `data`.
func apply_fine_anchors(store: FineGeoStore) -> void:
	_anchor_px.clear()
	for i in data.settlements.size():
		var id := str(data.settlements[i]["id"])
		if _landmarks.has(i) or not store.settlements.has(id):
			continue
		_anchor_px[i] = store.settlements[id]["px"]
		_ground_footprint(i)
	_hamlet_anchors = store.hamlets if store.hamlets.size() == data.hamlets.size() else PackedVector4Array()
	_forget_hamlet_exclusions()
	for index in _hamlet_nodes:
		_hamlet_dirty[index] = true
	_update_label_heights()


## Cercles d'exclusion de la végétation (x, y, rayon en px carte) : colonies et hameaux. VT : par
## colonie, le finage (`finage_radius_m`), sinon deux fois l'emprise réelle (au moins la zone d'une
## ville emblématique sans entrée dans `towns_1340.json`).
func vegetation_exclusions() -> PackedVector3Array:
	var result := PackedVector3Array()
	for i in data.settlements.size():
		var px: Vector2 = data.settlements[i]["px"]
		var radius := _finage_radius[i]
		if radius <= 0.0:
			radius = _model_radius[i] * 2.0
			if _landmarks.has(i):
				radius = maxf(radius, (_landmarks[i] as Vector3).z)
		result.append(Vector3(px.x, px.y, radius))
	for hamlet in data.hamlets:
		var hpx: Vector2 = hamlet["px"]
		result.append(Vector3(hpx.x, hpx.y, ModelLibrary.HAMLET_SCALE * 0.6))
	return result


# --- Accès pour les effets de vie (fumées, foule, rivières) ---------------------------------
# VT (ADR 0138) : plus de maquette ; signatures gardées pour les consommateurs (`LifeEffects`,
# `CampaignLife`, `FineGeoLayer`, `FolkScenes`), recâblés au lot G.


## Plus de maquette sur la carte (VT) : toujours null.
func model_holder(_i: int) -> Node3D:
	return null


## Emprise réelle au sol (unités monde) de la colonie `i` (`DEFAULT_FOOTPRINT_M` si inconnue).
func model_radius(i: int) -> float:
	return _model_radius[i] if i >= 0 and i < _model_radius.size() else DEFAULT_FOOTPRINT_M / 719.0


## Hauteur des toits (≈ `TOWN_TOP_M`) au-dessus du sol, en unités monde.
func model_top(i: int) -> float:
	return _model_top[i] if i >= 0 and i < _model_top.size() else TOWN_TOP_M / 719.0


## VT : les colonies sont à l'échelle réelle : 1.
func model_scale(_i: int) -> float:
	return 1.0


func model_scale_at(_i: int, _distance: float) -> float:
	return 1.0


## SZ4b : rayon réel au sol (unités) de la colonie `i` (emprise vers 1340), < 0 si inconnu.
func real_radius(i: int) -> float:
	return _real_radius[i] if i >= 0 and i < _real_radius.size() else -1.0


## Lot CV1 (croissance des maquettes) abandonné (VT) : sans effet, gardé pour `CampaignLife`.
func replace_models(_replacements: Array) -> void:
	pass


## Hameau brûlé (même tirage que `_build_hamlets`), pour les fumées d'incendie.
func hamlet_burned(h: int) -> bool:
	var hamlet: Dictionary = data.hamlets[h]
	var px: Vector2 = hamlet["px"]
	var seed_value := _hash(str(hamlet["name"]) + str(px))
	var devastation: float = _devastation.get(hamlet["province"], 0.0)
	return devastation >= BURN_THRESHOLD and float((seed_value / 7) % 100) < devastation


## Force une dévastation affichée (captures CV1 `--devastate`) et reconstruit les hameaux.
func override_devastation(values: Dictionary) -> void:
	for province_id in values:
		_devastation[province_id] = float(values[province_id])
	for index in _hamlet_nodes:
		_hamlet_dirty[index] = true


# --- Lot ZG6 : villes ordinaires à l'échelle réelle ------------------------------------------


func _setup_towns() -> void:
	towns = TownLayer.new()
	add_child(towns)
	var ids: Array = []
	for entry in data.settlements:
		ids.append(entry["id"])
	towns.setup(map_data, terrain, tiers, ids)
	_compute_footprints()
	landmark_cities = LandmarkCityLayer.new()
	add_child(landmark_cities)
	landmark_cities.setup(map_data, terrain, tiers, ids)
	town_far = TownFarLayer.new()
	add_child(town_far)
	town_far.setup(map_data, terrain, tiers, ids, [towns, landmark_cities], towns.data)


## VH4 : villes emblématiques 1:1.
func _update_landmark_cities(camera_distance: float) -> void:
	if landmark_cities != null:
		landmark_cities.update_view(camera_distance)


## ZG6 : villes ordinaires à l'échelle réelle autour de la caméra.
func _update_towns(camera_distance: float) -> void:
	if towns != null:
		towns.update_view(camera_distance)
