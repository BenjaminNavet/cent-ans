class_name SettlementLayer
extends Node3D

## Colonies et hameaux sur la carte de campagne (lot C6), rendu seulement :
## - paliers Europe et moyen : marqueurs (`settlement_icon.gdshader`, un `MultiMesh`), langage
##   unique du lot DA3 (ADR 0066, `SettlementMarkers`) : pictogramme peint selon le type, écu du
##   détenteur, taille selon le rang, dé-encombrement par distance caméra (données) ; noms des
##   cités (et des villes en se rapprochant) ;
## - palier près : maquettes 3D par type (`ModelLibrary.settlement_model`), posées sur la surface
##   exacte du terrain affiché et recalées quand une tuile change de niveau ; hameaux en
##   `MultiMesh` par tuile (orientation et variante déterministes, brûlés selon la dévastation de
##   la province) ; noms de toutes les colonies.
## Dé-encombrement écran (lot DA7d, ADR 0066) : marqueurs et noms posés par priorité (rang puis
## poids, `MarkerDeclutter`) ; ceux qui recouvrent un rectangle déjà posé cèdent la place (fondu
## du shader), sauf la capitale du joueur, la colonie sélectionnée et celle survolée. Recalcul
## seulement quand la caméra bouge nettement (paramètres dans `settlement_markers.json`).
## Picking écran : `pick_screen` → id, `select` → surbrillance + signal.

signal settlement_selected(id: String)

const KIND_INDEX := {"city": 0, "town": 1, "castle": 2, "abbey": 3, "village": 4}
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
## Rayon de picking d'un marqueur, en fraction de sa taille écran.
const PICK_ICON_FRACTION := 0.45
## Hauteur du centre du marqueur au-dessus du lieu, en fraction de sa taille (cf. shader).
const ICON_CENTER_LIFT := 0.42
## Nom au-dessus du marqueur (DA7d) : écart (px écran) entre le haut de l'emprise du
## pictogramme et le bas du texte mesuré (DC4), pour que le nom ne touche plus ses flèches.
const LABEL_GAP_PX := 2.0
## Proportion de hameaux brûlés = dévastation (%) × ce facteur (au-delà d'un seuil).
const BURN_THRESHOLD := 10.0
## Partage de l'écart entre deux maquettes voisines (voir `_fit_models`).
const FIT_WEIGHT := {"city": 3.0, "town": 2.0, "castle": 1.5, "abbey": 1.3, "village": 1.0}
const MIN_FIT_SCALE := 0.4
## Distance de retrait du marqueur de la colonie sélectionnée (toujours affiché, DA7d).
const SELECTED_UNTIL := 100000.0
## DC4 : une maquette dont le centre tombe à moins de ce facteur × son rayon du bord d'une voisine
## prioritaire (faubourg : Saint-Maximin sous Trèves) n'est pas affichée ; marqueur et nom restent.
const ABSORB_FACTOR := 0.5
## DC6c : maquette masquée aussi si elle recouvre une voisine prioritaire de plus de cette part du
## plus petit rayon (réduction bloquée au plancher `MIN_FIT_SCALE`).
const ABSORB_OVERLAP := 0.2

@export var tiers: ZoomTiers
## Échelle globale des marqueurs (tailles par rang dans `data/map/settlement_markers.json`).
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
## Lot DA3 : catalogue des marqueurs ; par colonie : rang, taille écran (px), distance de retrait.
var markers: SettlementMarkers
var _marker_rank: PackedInt32Array = PackedInt32Array()
var _marker_size: PackedFloat32Array = PackedFloat32Array()
var _marker_until: PackedFloat32Array = PackedFloat32Array()
## DC4 : position monde de chaque marqueur, ancre du `MultiMesh` (picking et dé-encombrement DA7d
## sans relire le relief).
var _marker_world: PackedVector3Array = PackedVector3Array()
## Écu affiché par colonie (faction), pour ne réécrire que ce qui change.
var _marker_holder: PackedStringArray = PackedStringArray()
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
var _models: Array = []  # par colonie : Node3D ou null
var _model_radius: PackedFloat32Array = PackedFloat32Array()
var _model_top: PackedFloat32Array = PackedFloat32Array()
## DC4 : rayon et hauteur des maquettes avant réduction, et facteur de réduction appliqué
## (`_fit_model`, recalculé après les ancrages fins et la croissance CV1).
var _base_radius: PackedFloat32Array = PackedFloat32Array()
var _base_top: PackedFloat32Array = PackedFloat32Array()
var _fit_scale: PackedFloat32Array = PackedFloat32Array()
## DC6c : place (unités) laissée par les voisines à chaque maquette (INF si aucune) : réduction à
## l'échelle effective (`SettlementFit`) ; paires de voisines candidates au masquage (clés
## `SettlementFit.pair_key` triées), positions de rendu et masquage courant.
var _room: PackedFloat32Array = PackedFloat32Array()
var _model_pairs: PackedInt64Array = PackedInt64Array()
var _pair_px: PackedVector2Array = PackedVector2Array()
var _absorbed: PackedByteArray = PackedByteArray()
var _models_root: Node3D
## Villes emblématiques (lot L1) : index de colonie → LandmarkModel (toujours visibles, LOD par
## portées de visibilité), sous `_landmarks_root`.
var _landmarks: Dictionary = {}
var _landmarks_root: Node3D
## ZG4 : maquettes masquées au palier « site ».
var _site_hidden: bool = false
var _labels_dirty: bool = false
var _labels_root: Node3D
var _hamlets_root: Node3D
var _selection_ring: MeshInstance3D
var _settlements_by_chunk: Dictionary = {}
var _hamlets_by_chunk: Dictionary = {}
var _hamlet_nodes: Dictionary = {}  # index de tuile → Node3D
var _hamlet_dirty: Dictionary = {}
var _burned_material: StandardMaterial3D
## Dévastation par province (0-100), lue depuis la simulation au rafraîchissement.
var _devastation: Dictionary = {}
var _colors: PackedColorArray = PackedColorArray()
var _declutter_timer := 0.0
var _weights := Vector3(-1, -1, -1)  # près, moyen, loin
var _camera_distance := 1000.0  # SZ4b : maquettes à la taille de carte avant la première vue
var _regrounded: Dictionary = {}
## ZG6 : villes ordinaires à l'échelle réelle (paliers vallée et site), voir `TownLayer`.
var towns: TownLayer
## VH4 (ADR 0078) : villes emblématiques à l'échelle 1:1 (format v2), voir `LandmarkCityLayer`.
var landmark_cities: LandmarkCityLayer
var _towns_version := -1
## Lot ZG5b : positions de rendu affinées (`fine_anchors.json`) des maquettes (index → Vector2)
## et des hameaux (x, y, z, déplacement), sans toucher aux positions de règles (`data`).
var _anchor_px: Dictionary = {}
var _hamlet_anchors: PackedVector4Array = PackedVector4Array()
## Lot SZ4 : échelle appliquée aux hameaux (1 au loin, taille réelle au palier vallée,
## `MapPropScale.hamlet_scale`) ; les tuiles sont reconstruites par pas de `rewrite_step`.
var _hamlet_scale := 1.0
## Lot SZ4b : maquettes des colonies à l'échelle continue (`MapPropScale.settlement_scale`).
## Par colonie : rayon réel au sol (unités, < 0 si inconnu), échelle appliquée, sol au centre de la
## pose réelle et point bas de l'emprise de carte (pose interpolée selon l'échelle, sans relire le
## relief à chaque pas de zoom).
var _real_radius: PackedFloat32Array = PackedFloat32Array()
var _model_scale: PackedFloat32Array = PackedFloat32Array()
var _ground_real: PackedFloat32Array = PackedFloat32Array()
var _ground_map: PackedFloat32Array = PackedFloat32Array()
## Échelle de référence (rapport par défaut) appliquée : réécriture par pas de `rewrite_step`.
var _settlement_scale_ref := -1.0
## RS-K2 : échelles `_effective_scale` mémorisées (à chaque pas de zoom, ~4 calculs par colonie
## ici et 2 par colonie pour les effets de vie). `_scale_memo` : à la distance
## `_scale_memo_distance` (vidée quand elle change) ; `_ratio_memo` : taille réelle (distance
## nulle). -1 = à calculer ; entrée remise à -1 quand la place, les rayons ou le rayon réel de la
## colonie changent (`_forget_scale`). Exagération commune mémorisée pour la dernière distance.
var _scale_memo: PackedFloat64Array = PackedFloat64Array()
var _ratio_memo: PackedFloat64Array = PackedFloat64Array()
var _scale_memo_distance := -1.0
## RS-K2 : tour de réécriture des maquettes (pas d'échelle) : maquettes par image, curseur et
## entrées restant à voir.
const PLACE_SLICE := 300
var _place_cursor := 0
var _place_left := 0
var _exaggeration_distance := -1.0
var _exaggeration_value := 1.0
## RS-K2 : hameaux écartés (ville emblématique, maquette) mémorisés : les tuiles de hameaux sont
## reconstruites à chaque recalage de relief, le test ne dépend que des maquettes et des ancrages.
## 0 : à calculer, 1 : posé, 2 : écarté ; graine de tirage par hameau (0 : à calculer).
var _hamlet_keep: PackedByteArray = PackedByteArray()
var _hamlet_seed: PackedInt64Array = PackedInt64Array()
## RS-K2 : échelle écran et boîte des marqueurs pour les décalages d'étiquettes, par image.
var _lift_frame := -1
var _lift_scale := 1.0
var _lift_box := 0.94


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
	_models.clear()
	_settlements_by_chunk.clear()
	_hamlets_by_chunk.clear()
	_hamlet_nodes.clear()
	_burned_material = StandardMaterial3D.new()
	_burned_material.albedo_color = Color(0.075, 0.062, 0.05)
	_burned_material.roughness = 1.0
	_models_root = Node3D.new()
	_models_root.name = "Models"
	add_child(_models_root)
	_landmarks.clear()
	_landmarks_root = Node3D.new()
	_landmarks_root.name = "Landmarks"
	add_child(_landmarks_root)
	_labels_root = Node3D.new()
	_labels_root.name = "Labels"
	add_child(_labels_root)
	_hamlets_root = Node3D.new()
	_hamlets_root.name = "Hamlets"
	add_child(_hamlets_root)
	var count := data.settlements.size()
	_colors.resize(count)
	_colors.fill(Color(0.6, 0.6, 0.6))
	_model_radius.resize(count)
	_model_top.resize(count)
	_fit_scale.resize(count)
	_fit_scale.fill(1.0)
	_base_radius.resize(count)
	_base_top.resize(count)
	_room.resize(count)
	_room.fill(INF)
	_absorbed.resize(count)
	_absorbed.fill(0)
	_model_pairs.clear()
	_real_radius.resize(count)
	_real_radius.fill(-1.0)
	_model_scale.resize(count)
	_model_scale.fill(1.0)
	_scale_memo.resize(count)
	_scale_memo.fill(-1.0)
	_ratio_memo.resize(count)
	_ratio_memo.fill(-1.0)
	_scale_memo_distance = -1.0
	_exaggeration_distance = -1.0
	_ground_real.resize(count)
	_ground_map.resize(count)
	_settlement_scale_ref = -1.0
	for i in count:
		var entry: Dictionary = data.settlements[i]
		var px: Vector2 = entry["px"]
		_register(_settlements_by_chunk, terrain.chunk_index_at(px.x, px.y), i)
		_build_model(i, entry)
		_build_label(i, entry)
	_fit_models()
	for i in data.hamlets.size():
		var hpx: Vector2 = data.hamlets[i]["px"]
		_register(_hamlets_by_chunk, terrain.chunk_index_at(hpx.x, hpx.y), i)
	_hamlet_seed.resize(data.hamlets.size())
	_hamlet_seed.fill(-1)
	_forget_hamlet_exclusions()
	_build_icons()
	_build_selection_ring()
	_setup_towns()
	if not terrain.chunk_surface_changed.is_connected(_on_chunk_surface_changed):
		terrain.chunk_surface_changed.connect(_on_chunk_surface_changed)
	stats = {"settlements": count, "hamlets": data.hamlets.size(), "models": _models.filter(func(m: Variant) -> bool: return m != null).size()}


static func _register(map: Dictionary, key: int, value: int) -> void:
	if key < 0:
		return
	if not map.has(key):
		map[key] = PackedInt32Array()
	map[key].append(value)


static func _hash(text: String) -> int:
	return absi(text.hash())


# --- Construction ------------------------------------------------------------------


func _build_model(i: int, entry: Dictionary) -> void:
	if _build_landmark(i, entry):
		return
	var model := ModelLibrary.settlement_model(str(entry["kind"]), _hash(str(entry["id"])) / 7)
	if model == null:
		_models.append(null)
		_model_radius[i] = 2.0
		_model_top[i] = 2.0
		_base_radius[i] = 2.0
		_base_top[i] = 2.0
		return
	var holder := Node3D.new()
	holder.name = str(entry["id"])
	var px: Vector2 = entry["px"]
	holder.position = Vector3(px.x, 0.0, px.y)
	holder.rotation.y = float(_hash(str(entry["id"]) + "yaw") % 628) / 100.0
	holder.add_child(model)
	var aabb := _model_aabb(model)
	_model_radius[i] = maxf(aabb.size.x, aabb.size.z) * 0.5
	_model_top[i] = aabb.end.y
	_base_radius[i] = _model_radius[i]
	_base_top[i] = _model_top[i]
	for geometry in model.find_children("*", "GeometryInstance3D", true, false):
		var g := geometry as GeometryInstance3D
		g.visibility_range_end = tiers.model_range
		g.visibility_range_end_margin = tiers.model_range * 0.15
	_models_root.add_child(holder)
	_models.append(holder)
	_ground_model(i)


## Ville emblématique (lot L1) : maquette dédiée à la place de la maquette générique.
func _build_landmark(i: int, entry: Dictionary) -> bool:
	var plan := LandmarkLibrary.for_settlement(str(entry["id"]))
	if plan.is_empty():
		return false
	var landmark := LandmarkModel.create(plan, terrain)
	if landmark == null:
		return false
	landmark.set_year(1337)
	_landmarks_root.add_child(landmark)
	_landmarks[i] = landmark
	_models.append(landmark)
	_model_radius[i] = landmark.core_radius
	_model_top[i] = 1.1
	_base_radius[i] = _model_radius[i]
	_base_top[i] = _model_top[i]
	return true


func _is_landmark(i: int) -> bool:
	return _landmarks.has(i)


## Hauteur de base d'une maquette (les villes emblématiques sont drapées par leur shader).
func _model_base_y(i: int) -> float:
	if _landmarks.has(i):
		return (_landmarks[i] as LandmarkModel).ground_height()
	return (_models[i] as Node3D).position.y


## VH4 : cercles des villes emblématiques encore sans ville 1:1 (format v2) : le plancher de
## caméra provisoire de ZG4b (`landmark_min_distance`) ne s'applique plus qu'à elles.
func landmark_floor_zones() -> PackedVector3Array:
	var zones := PackedVector3Array()
	for i in _landmarks:
		if landmark_cities != null and landmark_cities.is_enabled() and landmark_cities.has_city(str(data.settlements[i]["id"])):
			continue
		var landmark := _landmarks[i] as LandmarkModel
		zones.append(Vector3(landmark.position.x, landmark.position.z, landmark.zone_radius))
	return zones


## Cercles (x, z, rayon) des villes emblématiques, pour le zoom rapproché de la caméra.
func landmark_zones() -> PackedVector3Array:
	var zones := PackedVector3Array()
	for landmark in _landmarks.values():
		var center: Vector3 = (landmark as LandmarkModel).position
		zones.append(Vector3(center.x, center.z, (landmark as LandmarkModel).zone_radius))
	return zones


## DC4 (ADR 0082) : vrai si un point carte tombe dans l'emprise de la maquette d'une colonie
## (hameau de `hamlets.json` resté sur une place ajoutée depuis : il n'est pas posé).
func on_settlement_model(px: Vector2) -> bool:
	var index := terrain.chunk_index_at(px.x, px.y)
	var margin := ModelLibrary.HAMLET_SCALE * 0.3
	for dy in [-1, 0, 1]:
		for dx in [-1, 0, 1]:
			for j in _settlements_by_chunk.get(index + dy * terrain.chunks_x + dx, PackedInt32Array()):
				if _models[j] != null and not _landmarks.has(j) and px.distance_to(model_px(j)) < _model_radius[j] + margin:
					return true
	return false


## Vrai si un point carte est couvert par une ville emblématique (hameaux, végétation).
func covered_by_landmark(px: Vector2) -> bool:
	for landmark in _landmarks.values():
		if (landmark as LandmarkModel).covers(px):
			return true
	return false


## Réduit les maquettes trop proches d'une voisine (Paris / Vincennes / Saint-Denis) : l'écart
## entre deux colonies est partagé au prorata du poids du type, sans descendre sous
## `MIN_FIT_SCALE`. Étiquettes et picking utilisent le rayon réduit. DC4 (ADR 0082) : positions
## de rendu (ancrages fins) ; une ville emblématique ne se réduit pas, sa voisine prend tout
## l'écart restant ; recalcul à partir de la taille d'origine (appel répétable). Une maquette
## encore dans l'emprise d'une voisine prioritaire est masquée (`_update_absorption`).
## DC6c : `_model_radius` reste le rayon réduit à la taille de carte (hameaux, végétation, effets) ;
## de près, la réduction est recalculée sur le rayon rétréci (`SettlementFit.zoom_scale`, porté
## par `_model_scale`) et le masquage sur les rayons affichés, à chaque pas d'échelle.
func _fit_models() -> void:
	for i in data.settlements.size():
		_fit_model(i)
	_build_model_pairs()
	for i in _models.size():
		_place_model(i)
	_update_absorption()
	_forget_hamlet_exclusions()


## RS-K2 : maquettes ou ancrages changés : exclusions des hameaux à recalculer.
func _forget_hamlet_exclusions() -> void:
	if data == null:
		return
	_hamlet_keep.resize(data.hamlets.size())
	_hamlet_keep.fill(0)


## DC6c : paires de maquettes voisines qui peuvent se masquer (rayons à la taille de carte, les
## plus grands affichés : sur-ensemble valable à toute échelle).
func _build_model_pairs() -> void:
	_pair_px.resize(data.settlements.size())
	for i in data.settlements.size():
		_pair_px[i] = model_px(i)
	_model_pairs.clear()
	for i in data.settlements.size():
		_append_pairs_of(i, false)
	_model_pairs.sort()


## Ajoute les paires (`j` < `i`, ou toutes les voisines si `both`) de la maquette `i`.
func _append_pairs_of(i: int, both: bool) -> void:
	if _models[i] == null:
		return
	var px := _pair_px[i]
	var index := terrain.chunk_index_at(px.x, px.y)
	for dy in [-1, 0, 1]:
		for dx in [-1, 0, 1]:
			for j in _settlements_by_chunk.get(index + dy * terrain.chunks_x + dx, PackedInt32Array()):
				if j == i or (j > i and not both) or _models[j] == null:
					continue
				if px.distance_to(_pair_px[j]) < _model_radius[i] + _model_radius[j]:
					_model_pairs.append(SettlementFit.pair_key(i, j))


## Masque les maquettes dont le centre est dans l'emprise affichée d'une voisine prioritaire
## (placée avant dans l'ordre de priorité) à `ABSORB_FACTOR` × leur rayon affiché près.
func _update_absorption() -> void:
	if _pair_px.size() != _models.size():
		return
	var count := _models.size()
	var radius := PackedFloat32Array()
	radius.resize(count)
	var protected := PackedByteArray()
	protected.resize(count)
	for i in count:
		if _models[i] == null:
			radius[i] = 0.0
			protected[i] = 1
		elif _landmarks.has(i):
			radius[i] = _model_radius[i]
			protected[i] = 1
		else:
			radius[i] = _model_radius[i] * _model_scale[i]
			protected[i] = 0
	var result := SettlementFit.absorbed(_model_pairs, _pair_px, radius, protected, ABSORB_FACTOR, ABSORB_OVERLAP)
	for i in count:
		if result[i] != _absorbed[i]:
			_absorbed[i] = result[i]
			_apply_model_visibility(i)


## Maquette affichée sauf masquée (DC4/DC6c) ou remplacée par sa ville 1:1 (ZG6, SZ4b).
func _apply_model_visibility(i: int) -> void:
	var holder: Node3D = _models[i]
	if holder == null or _landmarks.has(i):
		return
	var town_shown := towns != null and towns.active and towns.is_shown(str(data.settlements[i]["id"]))
	holder.visible = _absorbed[i] == 0 and not town_shown


func _fit_model(i: int) -> void:
	var holder: Node3D = _models[i]
	if holder == null or _landmarks.has(i) or holder.get_child_count() == 0:
		return
	var px := model_px(i)
	var weight: float = FIT_WEIGHT.get(str(data.settlements[i]["kind"]), 1.0)
	var allowed := INF
	var index := terrain.chunk_index_at(px.x, px.y)
	for dy in [-1, 0, 1]:
		for dx in [-1, 0, 1]:
			var neighbor: int = index + dy * terrain.chunks_x + dx
			for j in _settlements_by_chunk.get(neighbor, PackedInt32Array()):
				if j == i or _models[j] == null:
					continue
				var d := px.distance_to(model_px(j))
				if _landmarks.has(j):
					allowed = minf(allowed, d - _model_radius[j])
				else:
					var other_weight: float = FIT_WEIGHT.get(str(data.settlements[j]["kind"]), 1.0)
					allowed = minf(allowed, d * weight / (weight + other_weight))
	_room[i] = allowed
	_forget_scale(i)
	var factor := SettlementFit.fit_factor(allowed, _base_radius[i], MIN_FIT_SCALE)
	if is_equal_approx(factor, _fit_scale[i]):
		return
	var model := holder.get_child(0) as Node3D
	model.scale *= factor / _fit_scale[i]
	_fit_scale[i] = factor
	_model_radius[i] = _base_radius[i] * factor
	_model_top[i] = _base_top[i] * factor
	_ground_model(i)


static func _model_aabb(root: Node3D) -> AABB:
	var result := AABB()
	var first := true
	for child in root.find_children("*", "MeshInstance3D", true, false):
		var mesh_instance := child as MeshInstance3D
		if mesh_instance.mesh == null:
			continue
		var local := mesh_instance.mesh.get_aabb()
		var xform := root.transform * _relative_transform(root, mesh_instance)
		var box := xform * local
		result = box if first else result.merge(box)
		first = false
	return result


static func _relative_transform(root: Node3D, node: Node3D) -> Transform3D:
	var xform := Transform3D.IDENTITY
	var current: Node = node
	while current != null and current != root:
		if current is Node3D:
			xform = (current as Node3D).transform * xform
		current = current.get_parent()
	return xform


## Pose la maquette sur la surface affichée : point le plus bas de l'emprise (les fondations des
## modèles descendent sous z = 0, rien ne flotte sur une pente).
## SZ4b : deux poses gardées (emprise de carte, emprise réelle), interpolées selon l'échelle.
func _ground_model(i: int) -> void:
	var holder: Node3D = _models[i]
	if holder == null or _landmarks.has(i):
		return
	var px := model_px(i)
	var center := terrain.surface_height_at(px.x, px.y)
	_ground_map[i] = _low_point(px, _model_radius[i] * 0.7, center)
	_ground_real[i] = _low_point(px, _model_radius[i] * _real_ratio(i) * 0.7, center)
	_place_model(i)


func _low_point(px: Vector2, radius: float, center: float) -> float:
	var low := center
	for k in 8:
		var angle := k * TAU / 8.0
		low = minf(low, terrain.surface_height_at(px.x + cos(angle) * radius, px.y + sin(angle) * radius))
	return low


## SZ4b : échelle de la maquette `i` à taille réelle (distance nulle), relative à la maquette
## réduite à la taille de carte (DC6c : la réduction DC4 est recalculée sur l'emprise réelle).
func _real_ratio(i: int) -> float:
	return _effective_scale(i, 0.0)


## RS-K2 : oublie les échelles mémorisées de la colonie `i` (place, rayons ou rayon réel changés).
func _forget_scale(i: int) -> void:
	if i < _scale_memo.size():
		_scale_memo[i] = -1.0
		_ratio_memo[i] = -1.0


## RS-K2 : exagération commune à `distance`, mémorisée pour la dernière distance demandée.
func _exaggeration_at(distance: float) -> float:
	if distance != _exaggeration_distance:
		_exaggeration_distance = distance
		_exaggeration_value = MapPropScale.shared().exaggeration(distance)
	return _exaggeration_value


## DC6c : rétrécissement SZ4b de la maquette d'origine `i` (taille réelle / taille d'origine,
## bornée) à la distance `distance`, sans réduction DC4.
func _effective_sigma(i: int, distance: float) -> float:
	var props := MapPropScale.shared()
	var ratio := props.settlement_default_ratio
	if i < _real_radius.size() and _real_radius[i] > 0.0 and _base_radius[i] > 0.0:
		ratio = _real_radius[i] / _base_radius[i]
	return props.settlement_scale_with(ratio, _exaggeration_at(distance))


## DC6c : échelle du porteur de la maquette `i` à la distance `distance` : rétrécissement SZ4b de
## la maquette d'origine (taille réelle / taille d'origine, bornée) puis réduction DC4 recalculée
## sur ce rayon (`SettlementFit.zoom_scale`), relative à la réduction de carte.
## RS-K2 : mémorisée (`_scale_memo`, `_ratio_memo`).
func _effective_scale(i: int, distance: float) -> float:
	if _landmarks.has(i):
		return 1.0
	if i >= _scale_memo.size():
		return SettlementFit.zoom_scale(_base_radius[i], _room[i], _effective_sigma(i, distance), MIN_FIT_SCALE)
	if distance == 0.0:
		var ratio := _ratio_memo[i]
		if ratio < 0.0:
			ratio = SettlementFit.zoom_scale(_base_radius[i], _room[i], _effective_sigma(i, 0.0), MIN_FIT_SCALE)
			_ratio_memo[i] = ratio
		return ratio
	if distance != _scale_memo_distance:
		_scale_memo_distance = distance
		_scale_memo.fill(-1.0)
	var s := _scale_memo[i]
	if s < 0.0:
		s = SettlementFit.zoom_scale(_base_radius[i], _room[i], _effective_sigma(i, distance), MIN_FIT_SCALE)
		_scale_memo[i] = s
	return s


## SZ4b : applique l'échelle courante de la maquette `i` (taille, pose sur le relief).
func _place_model(i: int) -> void:
	var holder: Node3D = _models[i]
	if holder == null or _landmarks.has(i):
		return
	var s := _effective_scale(i, _camera_distance)
	_model_scale[i] = s
	var ratio := _real_ratio(i)
	var along := clampf((s - ratio) / maxf(1.0 - ratio, 1e-4), 0.0, 1.0)
	var y := lerpf(_ground_real[i], _ground_map[i], along) - 0.03 * s
	# RS-K2 : chaque affectation propage la transformation à toute la maquette : seulement si
	# elle change (maquettes au plancher ou au plafond d'échelle pendant un zoom).
	var wanted_scale := Vector3.ONE * s
	if holder.scale != wanted_scale:
		holder.scale = wanted_scale
	var pose := holder.position
	if pose.y != y:
		pose.y = y
		holder.position = pose


## SZ4b : échelle des maquettes réécrite par pas de `rewrite_step` (≈ 0,3 ms pour 560 maquettes).
## RS-K2 (ADR 0051) : un pas lance un tour de réécriture étalé (`PLACE_SLICE` maquettes par image
## dans une image ouverte, reprise au curseur ; un nouveau pas relance un tour complet) ; masquage
## recalculé à chaque tranche.
func _update_settlement_scale(camera_distance: float) -> void:
	var props := MapPropScale.shared()
	var wanted := props.exaggeration(camera_distance)  # exagération commune
	if props.needs_rewrite(_settlement_scale_ref, wanted):
		_settlement_scale_ref = wanted
		_place_left = _models.size()
	if _place_left > 0:
		_place_slice(FrameBudget.in_frame())


## RS-K2 : tranche du tour de réécriture des maquettes et de leurs étiquettes (toutes si `sliced`
## est faux). Seules les étiquettes posées sur une maquette ordinaire dépendent de son échelle
## (villes emblématiques et palier moyen : non ; changements de palier et d'échelle verticale
## traités dans `update_view`).
func _place_slice(sliced: bool) -> void:
	var count := _models.size()
	if count == 0:
		_place_left = 0
		return
	var tp := Time.get_ticks_usec()  # RS-K2 : sous-sections du banc `--bench-probe`
	var near := _weights.x > 0.35
	var placed := 0
	var i := _place_cursor % count
	while _place_left > 0 and (not sliced or placed < PLACE_SLICE):
		if _models[i] != null and not _landmarks.has(i):
			_place_model(i)
			if near:
				_update_label_height(i, near)
			placed += 1
		_place_left -= 1
		i += 1
		if i == count:
			i = 0
	_place_cursor = i
	tp = PerfProbe.lap("settle/scale/place", tp)
	_update_absorption()  # DC6c : masquage aux rayons affichés (< 0,5 ms, à chaque tranche)
	PerfProbe.lap("settle/scale/absorb", tp)


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
		_marker_size[i] = markers.size_px(kind, rank)
		_marker_until[i] = markers.visible_until(kind, rank)
		_marker_holder[i] = ""
		var cell := markers.cell_of(markers.pictogram_for(kind, rank))
		var k := _icon_instance(i)
		_marker_world[i] = Vector3(px.x, map_data.surface_world_at(px.x, px.y) + 0.5, px.y)
		multimesh.set_instance_transform(k, Transform3D(Basis.IDENTITY, _marker_world[i]))
		# DA7d : b = affiché (1) / cède la place (0), a = instant du dernier changement (fondu).
		multimesh.set_instance_color(k, Color(-1.0, 1.0 if bool(entry.get("port", false)) else 0.0, 1.0, -1.0e4))
		multimesh.set_instance_custom_data(k, Color(cell, 0.0, _marker_size[i], _marker_until[i] / 100.0))
	_icon_material = ShaderMaterial.new()
	_icon_material.shader = preload("res://shaders/settlement_icon.gdshader")
	_icon_material.set_shader_parameter("atlas", markers.atlas)
	_icon_material.set_shader_parameter("atlas_grid", Vector2(markers.atlas_columns, markers.atlas_rows))
	_icon_material.set_shader_parameter("port_cell", float(markers.port_cell()))
	_icon_material.set_shader_parameter("shield_place", markers.placement("shield"))
	_icon_material.set_shader_parameter("badge_place", markers.placement("badge"))
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
	var missing := markers.shield_atlas == null
	for entry in data.settlements:
		var controller := str(entry["controller"])
		if controller != "" and not factions.has(controller):
			factions.append(controller)
			if not markers.shield_index.has(controller):
				missing = true
	if missing:
		factions.sort()
		markers.build_shield_atlas(factions)
		_icon_material.set_shader_parameter("shields", markers.shield_atlas)
		_icon_material.set_shader_parameter("shield_grid", Vector2(markers.shield_columns, markers.shield_rows))
		_marker_holder.fill("?")
	var multimesh := _icons.multimesh
	for i in data.settlements.size():
		var controller := str(data.settlements[i]["controller"])
		if controller == _marker_holder[i]:
			continue
		_marker_holder[i] = controller
		var k := _icon_instance(i)
		var color := multimesh.get_instance_color(k)
		color.r = float(markers.shield_of(controller))
		multimesh.set_instance_color(k, color)


## Taille écran (px) du marqueur d'une colonie (légende, étiquettes, picking).
func marker_size(i: int) -> float:
	return _marker_size[i] * icon_size_scale if i >= 0 and i < _marker_size.size() else 24.0


## Vrai si le marqueur de la colonie `i` est affiché à la distance caméra courante (palier de
## rang et dé-encombrement écran DA7d).
func marker_visible(i: int) -> bool:
	return marker_in_tier(i) and _marker_shown[i] == 1


## Vrai si le rang du marqueur `i` l'affiche à la distance caméra courante (avant dé-encombrement).
## La colonie sélectionnée reste affichée à toute distance.
func marker_in_tier(i: int) -> bool:
	return i >= 0 and i < _marker_until.size() and _camera_distance < _marker_until_of(i) and _weights.x < 0.65


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


## Couleurs des contrôleurs et dévastation des provinces. `color_of(faction_id) -> Color`.
func refresh(sim: Object, color_of: Callable) -> void:
	if data == null:
		return
	data.apply_live(sim)
	if not _landmarks.is_empty() and sim != null and sim.has_method("get_date_label"):
		var year := LandmarkModel.year_of(str(sim.call("get_date_label")))
		if year > 0:
			for landmark in _landmarks.values():
				(landmark as LandmarkModel).set_year(year)
			if landmark_cities != null:
				landmark_cities.set_year(year)
	_refresh_shields()
	_refresh_capital(sim)
	for i in data.settlements.size():
		var entry: Dictionary = data.settlements[i]
		var controller := str(entry["controller"])
		var color := Color(0.62, 0.6, 0.55)
		if controller != "" and color_of.is_valid():
			color = color_of.call(controller)
			color.a = 1.0
		if color != _colors[i]:
			_colors[i] = color
			if _models[i] != null:
				ModelLibrary.tint_banner(_models[i], color)
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
	var weights := Vector3(tiers.near_weight(camera_distance), tiers.medium_weight(camera_distance), tiers.far_weight(camera_distance))
	# ZG4 : au palier « site » (~1 km, jusqu'à 200 m), les maquettes à la loupe (colonies ×3-7,
	# villes emblématiques ×3,5) dépasseraient les collines : masquées en attendant les villes à
	# l'échelle réelle (ZG6, VH4).
	var site := tiers.site_weight(camera_distance) > 0.5
	if weights != _weights or site != _site_hidden:
		_weights = weights
		_site_hidden = site
		# DA3 : marqueurs aux paliers Europe et moyen (dé-encombrés par le shader), retirés au
		# profit des maquettes au palier près.
		var icon_alpha := 1.0 - weights.x
		_icon_material.set_shader_parameter("alpha", icon_alpha)
		_icons.visible = icon_alpha > 0.01
		# SZ4b : maquettes à leur taille réelle sous le palier vallée, masquées une par une quand
		# leur ville 1:1 est affichée (`_update_model_visibility`).
		_models_root.visible = weights.x > 0.35
		# SZ4 : hameaux à leur taille réelle sous le palier comté, gardés au palier site.
		_hamlets_root.visible = weights.x > 0.35
		_landmarks_root.visible = not site
		# SZ6 : les hauteurs d'étiquettes ne dépendent des poids que par le palier près et les
		# villes 1:1 : pas de recalcul des 570 étiquettes à chaque image d'un zoom.
		var label_state := Vector2i(int(weights.x > 0.35), 0)  # RS-K : indépendant des villes 1:1
		if label_state != _label_state:
			_update_label_heights()
		_declutter_timer = 0.0
		_declutter_force = true
	if MapData.vertical_scale() != _label_scale:
		_labels_dirty = false
		var tv := Time.get_ticks_usec()
		_update_label_heights()  # ZG4 : toutes les étiquettes du palier moyen suivent l'échelle
		PerfProbe.lap("settle/labels/vscale", tv)  # RS-K3
	elif _labels_dirty:
		_labels_dirty = false
		# SZ6 : seules les colonies des morceaux recalés (hauteur de leur maquette ou de leur ville
		# emblématique) changent.
		var near := _weights.x > 0.35
		for index: int in _label_chunks:
			for i in _settlements_by_chunk.get(index, PackedInt32Array()):
				_update_label_height(i, near)
		_label_chunks.clear()
	if not is_equal_approx(camera_distance, _icon_distance) and _icon_material != null:
		_icon_distance = camera_distance
		_icon_material.set_shader_parameter("camera_distance", camera_distance)
	tp = PerfProbe.lap("settle/labels", tp)
	_update_settlement_scale(camera_distance)
	tp = PerfProbe.lap("settle/scale", tp)
	_update_towns(camera_distance)
	tp = PerfProbe.lap("settle/towns", tp)
	_update_landmark_cities(camera_distance)
	tp = PerfProbe.lap("settle/landmarks", tp)
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
			declutter()
	_update_declutter_fade()
	PerfProbe.lap("settle/declutter", tp)


func _on_chunk_surface_changed(index: int) -> void:
	for i in _settlements_by_chunk.get(index, PackedInt32Array()):
		_ground_model(i)
	if _hamlet_nodes.has(index):
		_hamlet_dirty[index] = true
	# ZG4 : hauteurs des étiquettes une fois par image (et non à chaque morceau recalé).
	_labels_dirty = true
	_label_chunks[index] = true


## SZ6 : morceaux recalés depuis la dernière mise à jour des étiquettes ; état (palier près,
## villes 1:1) de la dernière mise à jour complète.
var _label_chunks: Dictionary = {}
var _label_state := Vector2i(-1, -1)
var _label_scale := -1.0


## Hauteur des étiquettes : au-dessus de la maquette (près) ou de l'icône (moyen).
func _update_label_heights() -> void:
	var near := _weights.x > 0.35
	_label_state = Vector2i(int(near), 0)
	_label_scale = MapData.vertical_scale()
	_label_chunks.clear()
	for i in _labels.size():
		_update_label_height(i, near)


func _update_label_height(i: int, near: bool) -> void:
	var label := _labels[i]
	var px: Vector2 = data.settlements[i]["px"]
	if near and _models[i] != null:
		# RS-K2 : position écrite en une fois (une propagation de transformation par étiquette).
		var label_at := label.position
		if not _landmarks.has(i):
			var model_at := model_px(i)  # ZG5b : au-dessus de la maquette ancrée
			# SZ4b : au-dessus de la maquette à l'échelle courante ; au sol pour une ville 1:1.
			var s := _model_scale[i]
			var ratio := _real_ratio(i)
			var lift := 0.12 + 0.68 * clampf((s - ratio) / maxf(1.0 - ratio, 1e-4), 0.0, 1.0)
			label_at = Vector3(model_at.x, _model_base_y(i) + _model_top[i] * s + lift, model_at.y)
		else:
			label_at.y = _model_base_y(i) + _model_top[i] + 0.8
		if label.position != label_at:
			label.position = label_at
		label.offset = Vector2.ZERO
	else:
		# Palier moyen : au-dessus de l'icône (décalage en pixels écran).
		label.position = Vector3(px.x, map_data.surface_world_at(px.x, px.y) + 0.5, px.y)
		label.offset = Vector2(0.0, _label_lift_px(i))


## Opacité d'une étiquette selon le type et le palier (cité : moyen et près ; ville : moyen
## rapproché ; autres : près seulement).
func _label_alpha(kind: String) -> float:
	match kind:
		"city":
			return clampf(_weights.x + _weights.y, 0.0, 1.0)
		"town":
			var t := 1.0 - smoothstep(tiers.town_label_distance * 0.85, tiers.town_label_distance * 1.15, _camera_distance)
			return clampf(maxf(_weights.x, _weights.y * t), 0.0, 1.0)
		_:
			return _weights.x


## Lot DA7d (ADR 0066) : dé-encombrement écran unique des marqueurs et des noms. Colonie par
## colonie dans l'ordre de priorité (épinglés, puis rang, poids, ordre des données) : le marqueur,
## puis son nom. Un marqueur qui recouvre un rectangle déjà posé cède la place (fondu du shader)
## et son nom avec lui ; un nom qui recouvre un rectangle posé est masqué. Rectangles des noms
## mesurés avec la police (DC4, `_label_screen_rect`) ; grille spatiale (`MarkerDeclutter`),
## coût linéaire avec ~1 200 colonies. Remplace le masquage des seuls noms de DC4.
func declutter() -> void:
	var camera := get_viewport().get_camera_3d() if is_inside_tree() else null
	if camera == null or data == null or _priority_order.is_empty():
		return
	var t0 := Time.get_ticks_usec()
	var tp := t0  # RS-K3 : sous-sections du banc `--bench-probe`
	_declutter_force = false
	_declutter_camera = _camera_state(camera)
	var screen := get_viewport().get_visible_rect()
	var spill := float(markers.declutter_value("label_screen_margin", 0.2))
	var label_screen := screen.grow_individual(screen.size.x * spill, screen.size.y * spill, screen.size.x * spill, screen.size.y * spill)
	var marker_margin := float(markers.declutter_value("marker_margin_px", 2.0))
	var label_margin := float(markers.declutter_value("label_margin_px", declutter_margin))
	var icons_on := _icons != null and _icons.visible and _weights.x < 0.65
	_placer.reset(_max_marker_px * icon_size_scale + marker_margin * 2.0)
	# CV3-0 (#7) : les plaques/étendards d'armée sont réservés avant les colonies (déjà placés,
	# stables d'une frame à l'autre) ; un marqueur ou un nom de colonie qui les recouvre cède
	# la place, au lieu de se superposer (étiquettes qui se chevauchaient au pied de l'armée).
	if label_obstacles.is_valid():
		for rect: Rect2 in label_obstacles.call(camera):
			_placer.try_place(rect, -2, true)
	var pins := _pinned_indices()
	var box_fraction := markers.marker_box()
	# ZG4 : paliers vallée / site (vue rasante) : seulement les colonies proches, l'horizon ne se
	# couvre pas de noms.
	var close_w := tiers.valley_weight(_camera_distance) if tiers != null else 0.0
	var label_range := tiers.close_label_range_factor * _camera_distance if tiers != null else INF
	var alpha_by_kind := {}
	for kind in KIND_INDEX:
		alpha_by_kind[kind] = _label_alpha(kind)
	var camera_at := camera.global_position
	var scale := _label_screen_scale(camera)
	var sequence := PackedInt32Array(pins)
	for i in _priority_order:
		if not pins.has(i):
			sequence.append(i)
	tp = PerfProbe.lap("settle/declutter/prep", tp)
	for i in sequence:
		var has_marker := icons_on and marker_in_tier(i)
		var shown := true
		_marker_screen[i] = Vector2(-1.0e6, -1.0e6)
		if has_marker and not camera.is_position_behind(_marker_world[i]):
			var marker_rect := _marker_rect(i, camera, marker_margin, box_fraction)
			# Hors de l'écran élargi : rien à départager (recalcul dès que la caméra bouge).
			if label_screen.intersects(marker_rect):
				shown = _placer.try_place(marker_rect, i, pins.has(i))
				if shown:
					_marker_screen[i] = marker_rect.get_center()
		_set_marker_shown(i, shown)
		var alpha: float = alpha_by_kind.get(_label_kind[i], _weights.x)
		if not shown or alpha < 0.02:
			_show_label(i, false, alpha)
			continue
		var at := _labels[i].global_position
		if (close_w > 0.5 and camera_at.distance_to(at) > label_range) or camera.is_position_behind(at):
			_show_label(i, false, alpha)
			continue
		var rect := _label_screen_rect(i, camera, scale, label_margin)
		_show_label(i, label_screen.intersects(rect) and _placer.try_place(rect, i), alpha)
	PerfProbe.lap("settle/declutter/run", tp)
	last_declutter_ms = float(Time.get_ticks_usec() - t0) / 1000.0


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


## DA7d : décalage du nom `i` au-dessus de son marqueur (px du `Label3D`) : haut de l'emprise du
## pictogramme + demi-hauteur du texte mesuré + `LABEL_GAP_PX`, ramené à l'échelle écran.
func _label_lift_px(i: int) -> float:
	# RS-K2 : caméra et boîte lues une fois par image (570 étiquettes par recalcul).
	var frame := Engine.get_process_frames()
	if frame != _lift_frame:
		_lift_frame = frame
		var camera := get_viewport().get_camera_3d() if is_inside_tree() else null
		_lift_scale = _label_screen_scale(camera) if camera != null else 1.0
		_lift_box = markers.marker_box().y if markers != null else 0.94
	var marker_top := marker_size(i) * (ICON_CENTER_LIFT + _lift_box * 0.5)
	return (marker_top + LABEL_GAP_PX) / maxf(_lift_scale, 0.1) + _label_text_size(i).y * 0.5


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
	if bool(markers.declutter_value("pin_hovered", true)) and _weights.x < 0.65 and is_inside_tree():
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


## Lot DA7d : rectangle écran de l'emprise opaque du marqueur `i` (marge `margin` en px).
func _marker_rect(i: int, camera: Camera3D, margin: float, box_fraction: Vector2 = Vector2(0.8, 0.8)) -> Rect2:
	var size := marker_size(i)
	var center := camera.unproject_position(_marker_world[i]) - Vector2(0.0, size * ICON_CENTER_LIFT)
	var box := box_fraction * size + Vector2(margin, margin) * 2.0
	return Rect2(center - box * 0.5, box)


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
		if icons_on and marker_visible(i):
			if not camera.is_position_behind(_marker_world[i]):
				var rect := _marker_rect(i, camera, 0.0, markers.marker_box())
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


## Construit tout de suite hameaux et maquettes voulus (captures, tests).
func flush() -> void:
	var saved := max_hamlet_builds_per_frame
	max_hamlet_builds_per_frame = 1 << 20
	FrameBudget.unlimited = true
	_update_hamlets()
	FrameBudget.unlimited = false
	max_hamlet_builds_per_frame = saved
	for landmark: LandmarkModel in _landmarks.values():  # ZG4 : cuissons étalées terminées
		landmark.flush_bake()
	if towns != null:  # ZG6 : villes 1:1 autour de la caméra
		towns.flush()
		_update_towns(_camera_distance)
	if landmark_cities != null:  # VH4 : villes emblématiques 1:1
		landmark_cities.flush()
		_update_landmark_cities(_camera_distance)
	if _place_left > 0:  # RS-K2 : tour de réécriture des maquettes terminé
		_place_slice(false)
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
	# PB3g : hauteurs (centre + 4 points de l'emprise) en un seul appel groupé.
	var pending: Array = []
	var points := PackedVector2Array()
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
			seed_value = _hash(str(hamlet["name"]) + str(px))
			if memo:
				_hamlet_seed[h] = seed_value
		px = hamlet_px(h)  # ZG5b : ancrage fin (tirages inchangés)
		if keep == 0:
			if on_settlement_model(px):
				if memo:
					_hamlet_keep[h] = 2
				continue
			if memo:
				_hamlet_keep[h] = 1
		var variant := seed_value % meshes.size()
		var devastation: float = _devastation.get(hamlet["province"], 0.0)
		var burned := devastation >= BURN_THRESHOLD and float((seed_value / 7) % 100) < devastation
		var key := variant * 2 + (1 if burned else 0)
		var yaw := float((seed_value / 13) % 628) / 100.0
		var scale := ModelLibrary.HAMLET_SCALE * (0.85 + float((seed_value / 17) % 30) / 100.0)
		points.append(px)
		for k in 4:
			var angle := k * TAU / 4.0 + yaw
			points.append(Vector2(px.x + cos(angle) * scale * 0.4, px.y + sin(angle) * scale * 0.4))
		pending.append([key, px, yaw, scale])
	tp = PerfProbe.lap("settle/hamlets/prep", tp)
	var heights := terrain.surface_heights_at(points)
	tp = PerfProbe.lap("settle/hamlets/heights", tp)
	for p in pending.size():
		var key: int = pending[p][0]
		var px: Vector2 = pending[p][1]
		var center := heights[p * 5]
		var low := center
		for k in range(1, 5):
			low = minf(low, heights[p * 5 + k])
		if not groups.has(key):
			groups[key] = []
		# SZ4 : (x, z, lacet, échelle de carte, sol au centre, sol le plus bas de l'emprise de carte).
		groups[key].append(PackedFloat32Array([px.x, px.y, pending[p][2], pending[p][3], center, low]))
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


## Colonie sous un point écran (icône au palier moyen, maquette au palier près), "" sinon.
func pick_screen(screen_position: Vector2) -> String:
	return str(pick_screen_scored(screen_position).get("id", ""))


## Q2 : comme `pick_screen`, avec `score` (distance au centre / rayon de prise, 0 = en plein
## centre) pour départager une colonie et une armée sous le même clic ; {} si rien.
func pick_screen_scored(screen_position: Vector2) -> Dictionary:
	var camera := get_viewport().get_camera_3d() if is_inside_tree() else null
	if camera == null or data == null:
		return {}
	var near := _weights.x > 0.35
	var icons := _weights.x < 0.65
	var best := ""
	var best_score := INF
	var right := camera.global_transform.basis.x
	var eye := camera.global_position
	var model_range_sq := tiers.model_range * tiers.model_range
	for i in data.settlements.size():
		if near and _models[i] != null and (_models[i] as Node3D).visible:  # DC4 : pas les absorbées
			var holder: Node3D = _models[i]
			if eye.distance_squared_to(holder.position) > model_range_sq:
				continue  # maquette hors de sa portée de visibilité
			# Rayon effectif = rayon d'origine × réduction DC4 (`_model_radius`) × échelle SZ4b.
			var zoom_scale := model_scale(i)
			var center := Vector3(holder.position.x, _model_base_y(i) + _model_top[i] * zoom_scale * 0.4, holder.position.z)
			if camera.is_position_behind(center):
				continue
			var screen_center := camera.unproject_position(center)
			var edge := camera.unproject_position(center + right * _model_radius[i] * zoom_scale)
			var radius_px := maxf(screen_center.distance_to(edge), 8.0)
			var d := screen_center.distance_to(screen_position)
			if d < radius_px and d / radius_px < best_score:
				best_score = d / radius_px
				best = str(data.settlements[i]["id"])
		elif icons and marker_visible(i):  # DA7d : pas les marqueurs cédés
			var world := _marker_world[i]
			if camera.is_position_behind(world):
				continue
			var size := marker_size(i)
			var radius := size * PICK_ICON_FRACTION
			var center_px := camera.unproject_position(world) - Vector2(0.0, size * ICON_CENTER_LIFT)
			var d_icon := center_px.distance_to(screen_position)
			if d_icon < radius and d_icon / radius < best_score:
				best_score = d_icon / radius
				best = str(data.settlements[i]["id"])
	return {"id": best, "score": best_score} if best != "" else {}


## Sélectionne une colonie ("" = aucune) : surbrillance de l'icône, anneau au sol, signal.
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
	var holder: Node3D = _models[index] if index >= 0 else null
	var show := index >= 0 and holder != null and _weights.x > 0.35
	_selection_ring.visible = show
	if show:
		var radius := _model_radius[index] * model_scale(index) * 1.1
		_selection_ring.position = Vector3(holder.position.x, _model_base_y(index) + 0.15, holder.position.z)
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


## Lot ZG5b : maquettes (hors villes emblématiques) et hameaux posés aux ancrages fins de
## `fine_anchors.json` (déplacés de ≤ 300 m hors des lits et des pentes fortes), puis recalés
## sur la surface affichée. Icônes, étiquettes du palier moyen, picking et règles gardent les
## positions de `data`.
func apply_fine_anchors(store: FineGeoStore) -> void:
	_anchor_px.clear()
	for i in data.settlements.size():
		var id := str(data.settlements[i]["id"])
		if _landmarks.has(i) or not store.settlements.has(id) or _models[i] == null:
			continue
		var p: Vector2 = store.settlements[id]["px"]
		_anchor_px[i] = p
		var holder := _models[i] as Node3D
		holder.position.x = p.x
		holder.position.z = p.y
		_ground_model(i)
	_fit_models()  # DC4 : écarts recalculés aux positions de rendu
	_hamlet_anchors = store.hamlets if store.hamlets.size() == data.hamlets.size() else PackedVector4Array()
	_forget_hamlet_exclusions()
	for index in _hamlet_nodes:
		_hamlet_dirty[index] = true
	_update_label_heights()


## Cercles d'exclusion de la végétation (x, y, rayon en px carte) : colonies et hameaux.
func vegetation_exclusions() -> PackedVector3Array:
	var result := PackedVector3Array()
	for i in data.settlements.size():
		var px: Vector2 = data.settlements[i]["px"]
		result.append(Vector3(px.x, px.y, _model_radius[i] * 1.1 + 0.5))
	for landmark in _landmarks.values():
		var center: Vector3 = (landmark as LandmarkModel).position
		result.append(Vector3(center.x, center.z, (landmark as LandmarkModel).zone_radius))
	for hamlet in data.hamlets:
		var hpx: Vector2 = hamlet["px"]
		result.append(Vector3(hpx.x, hpx.y, ModelLibrary.HAMLET_SCALE * 0.6))
	return result


# --- Lot CV1 : accès pour la campagne vivante (croissance, fumées) -------------------------


## Support (Node3D posé sur le relief) de la maquette de la colonie `i`, null sans maquette.
func model_holder(i: int) -> Node3D:
	if _is_landmark(i):
		return null  # ville emblématique (L1) : pas de croissance ni de surcouche génériques
	return _models[i] if i >= 0 and i < _models.size() else null


## Rayon au sol et hauteur (unités monde) de la maquette de la colonie `i`.
func model_radius(i: int) -> float:
	return _model_radius[i] if i >= 0 and i < _model_radius.size() else 2.0


func model_top(i: int) -> float:
	return _model_top[i] if i >= 0 and i < _model_top.size() else 2.0


## SZ4b : échelle courante de la maquette `i` (1 au loin, taille réelle au palier vallée).
func model_scale(i: int) -> float:
	return _model_scale[i] if i >= 0 and i < _model_scale.size() else 1.0


## DC6c : rayon affiché de la maquette `i` (réduction DC4 à l'échelle courante × échelle SZ4b),
## 0 sans maquette ; `shown_fit` : réduction DC4 à l'échelle courante (1 = taille pleine) ;
## `model_absorbed` : maquette masquée sous une voisine prioritaire.
func shown_radius(i: int) -> float:
	if i < 0 or i >= _models.size() or _models[i] == null:
		return 0.0
	return _model_radius[i] if _landmarks.has(i) else _model_radius[i] * _model_scale[i]


func shown_fit(i: int) -> float:
	if i < 0 or i >= _models.size() or _models[i] == null or _landmarks.has(i):
		return 1.0
	return _fit_scale[i] * _model_scale[i] / maxf(_effective_sigma(i, _camera_distance), 1e-6)


func model_absorbed(i: int) -> bool:
	return i >= 0 and i < _absorbed.size() and _absorbed[i] != 0


## SZ4b : échelle de la maquette `i` à la distance de caméra `distance` (effets de `LifeEffects`).
func model_scale_at(i: int, distance: float) -> float:
	if i < 0 or i >= _models.size() or _landmarks.has(i):
		return 1.0
	return _effective_scale(i, distance)


## SZ4b : rayon réel au sol (unités) de la colonie `i` (emprise vers 1340), < 0 si inconnu.
func real_radius(i: int) -> float:
	return _real_radius[i] if i >= 0 and i < _real_radius.size() else -1.0


## Remplace la maquette de la colonie `i` (lot CV1 : croissance) ; `model` est déjà à l'échelle
## monde. Garde position, orientation, portée de visibilité et teinte de bannière ; l'écart
## aux voisines (`_fit_models`) est réappliqué.
func replace_model(i: int, model: Node3D) -> void:
	replace_models([[i, model]])


## OMR-R2 : remplacements groupés `[[i, maquette], …]` (croissance CV1 au lancement : ~500
## maquettes). Chaque maquette est posée et ajustée, puis paires, masquage et hauteurs
## d'étiquettes sont recalculés une seule fois (au lieu d'une passe sur toutes les colonies par
## maquette, ≈ 3 s au lancement de la carte Oural–Méditerranée).
func replace_models(replacements: Array) -> void:
	var replaced := {}
	for entry: Array in replacements:
		var i: int = entry[0]
		var model: Node3D = entry[1]
		var holder: Node3D = model_holder(i)
		if holder == null or model == null:
			continue
		for child in holder.get_children():
			holder.remove_child(child)
			child.queue_free()
		holder.add_child(model)
		var aabb := _model_aabb(model)
		_model_radius[i] = maxf(aabb.size.x, aabb.size.z) * 0.5
		_model_top[i] = aabb.end.y
		_base_radius[i] = _model_radius[i]
		_base_top[i] = _model_top[i]
		_fit_scale[i] = 1.0
		_model_scale[i] = 1.0
		_forget_scale(i)
		for geometry in model.find_children("*", "GeometryInstance3D", true, false):
			var g := geometry as GeometryInstance3D
			g.visibility_range_end = tiers.model_range
			g.visibility_range_end_margin = tiers.model_range * 0.15
		if i < _colors.size():
			ModelLibrary.tint_banner(holder, _colors[i])
		_ground_model(i)
		_fit_model(i)
		_place_model(i)
		replaced[i] = true
	if replaced.is_empty():
		return
	_forget_hamlet_exclusions()
	if _pair_px.size() == _models.size():
		_refresh_pairs_of_all(replaced)
	_update_absorption()
	_update_label_heights()


## OMR-R2 : paires des maquettes `replaced` (clés) recalculées en une passe ; une paire de deux
## maquettes remplacées n'est ajoutée qu'une fois (par la plus petite).
func _refresh_pairs_of_all(replaced: Dictionary) -> void:
	var kept := PackedInt64Array()
	for key in _model_pairs:
		if not replaced.has(key >> 16) and not replaced.has(key & 0xFFFF):
			kept.append(key)
	_model_pairs = kept
	for i: int in replaced:
		if _models[i] == null:
			continue
		var px := _pair_px[i]
		var index := terrain.chunk_index_at(px.x, px.y)
		for dy in [-1, 0, 1]:
			for dx in [-1, 0, 1]:
				for j in _settlements_by_chunk.get(index + dy * terrain.chunks_x + dx, PackedInt32Array()):
					if j == i or _models[j] == null or (j < i and replaced.has(j)):
						continue
					if px.distance_to(_pair_px[j]) < _model_radius[i] + _model_radius[j]:
						_model_pairs.append(SettlementFit.pair_key(i, j))
	_model_pairs.sort()


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
	_compute_real_radii()
	landmark_cities = LandmarkCityLayer.new()
	add_child(landmark_cities)
	landmark_cities.setup(map_data, terrain, tiers, ids)


## VH4 : villes emblématiques 1:1 et fondu de leur maquette L1/L2 (tramage).
func _update_landmark_cities(camera_distance: float) -> void:
	if landmark_cities == null:
		return
	landmark_cities.update_view(camera_distance)
	for i in _landmarks:
		var id := str(data.settlements[i]["id"])
		if landmark_cities.has_city(id):
			(_landmarks[i] as LandmarkModel).set_fade(landmark_cities.fade(id))


## SZ4b : rayon réel de chaque colonie : rayon bâti vers 1340 (`towns_1340.json`, lot ZG6) ×
## `settlement_footprint_gain` ; les maquettes rétrécissent vers ce rayon.
func _compute_real_radii() -> void:
	if towns == null or towns.data == null:
		return
	var gain := MapPropScale.shared().settlement_footprint_gain
	var mpu := towns.data.meters_per_unit
	for i in data.settlements.size():
		var id := str(data.settlements[i]["id"])
		if not towns.data.has_town(id):
			continue
		var built_ha := float(towns.data.towns[id].get("built_ha", 0.0))
		if built_ha > 0.0:
			_real_radius[i] = sqrt(built_ha * 10000.0 / PI) * gain / mpu
			_forget_scale(i)
	for i in _models.size():
		_ground_model(i)


## Rendu 1:1 aux paliers vallée / site. Tant qu'il est actif, les maquettes à la loupe des
## colonies ordinaires sont masquées (au loin, une ville vraie de 1340 n'est qu'une tache : les
## maquettes géantes à l'horizon disparaissent) ; les villes emblématiques restent au lot VH.
func _update_towns(camera_distance: float) -> void:
	if towns == null:
		return
	towns.update_view(camera_distance)
	if towns.version == _towns_version:
		return
	_towns_version = towns.version
	var tp := Time.get_ticks_usec()
	# RS-K : seules les colonies dont la ville 1:1 vient d'apparaître ou de disparaître (toutes
	# les ~570 maquettes coûtaient jusqu'à 25 ms à chaque ville construite).
	# La bascule d'activité ne touche que les colonies aux villes construites ; les hauteurs
	# d'étiquettes ne dépendent pas des villes 1:1 (plus de recalcul des ~570 étiquettes ici).
	var changes := towns.take_changes()
	if bool(changes["all"]):
		_update_model_visibility()
	else:
		for id: String in changes["ids"]:
			var i := int(data.index_by_id.get(id, -1))
			if i >= 0 and i < _models.size():
				_apply_model_visibility(i)
	PerfProbe.lap("town/models", tp)  # RS-K


## SZ4b : maquette masquée seulement quand la ville 1:1 de sa colonie est affichée (ZG6) ; ailleurs
## (hors du rayon de chargement, ville en cours de construction), la maquette à taille réelle reste.
func _update_model_visibility() -> void:
	for i in _models.size():
		_apply_model_visibility(i)
