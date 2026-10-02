class_name TownMaquetteLayer
extends Node3D

## Lot GC2 (ADR 0158) : villes stylisées de la carte de campagne. Chaque lieu est une maquette du
## kit (`assets/models/settlements/<type>[_<famille>]_<a|b>.glb`) posée à sa vraie position, à
## taille monde constante par type (`data/art/town_maquettes.json`, voir `TownMaquetteData`) :
## - un `MultiMesh` par (modèle, tuile de carte de `tile_size` unités) : culling par tuile, portée
##   par type (fondu sur `fade_margin` selon la distance du rig) ;
## - famille d'architecture par province (culture, région, religion), repli sur l'Ouest tant que
##   le modèle de la famille manque ; variante et lacet tirés par id de lieu ;
## - un lieu dont le disque touche celui d'un lieu plus important est réduit (`collision`) ;
## - bannières (matériau `Banner`) à la couleur du contrôleur, par donnée d'instance ;
## - posé au sol affiché (hauteur en mètres lue une fois, remise à l'échelle verticale courante
##   par morceau recalé puis par tranches) ;
## - les villes emblématiques (`data/landmarks/`) gardent leur `LandmarkModel`, grossi de
##   `landmark_scale`.
## Style par défaut (`map.town_style`), `--town-style=real` rend les villes 1:1. Purement visuel.

const BANNER_SHADER := preload("res://shaders/maquette_banner.gdshader")
## Lieux reposés par image pendant un tour complet (changement d'échelle verticale).
const POSE_SLICE := 400

var stats: Dictionary = {}

var _map_data: MapData
var _terrain: TerrainBuilder
var _layer: SettlementLayer
## Par lieu (index de `SettlementData.settlements`).
var _factor: PackedFloat32Array = PackedFloat32Array()  # réduction de collision (1 = taille de base)
var _radius: PackedFloat32Array = PackedFloat32Array()  # demi-largeur (unités monde)
var _gain: PackedFloat32Array = PackedFloat32Array()  # gain de taille selon le poids du lieu
var _absorbed: PackedByteArray = PackedByteArray()  # 1 : noyé dans une ville emblématique, sans maquette
var _top: PackedFloat32Array = PackedFloat32Array()  # hauteur au-dessus du sol (unités monde)
var _range: PackedFloat32Array = PackedFloat32Array()  # portée (distance du rig)
var _kind: PackedByteArray = PackedByteArray()  # index dans `TownMaquetteData.KINDS`
var _yaw: PackedFloat32Array = PackedFloat32Array()
var _model_scale: PackedFloat32Array = PackedFloat32Array()  # échelle du modèle du kit (0 : pas de modèle)
var _model_of: PackedInt32Array = PackedInt32Array()  # index dans `_model_list`, -1 sinon
var _tile_of: PackedInt32Array = PackedInt32Array()  # index dans `_tiles`, -1 sinon
var _slot: PackedInt32Array = PackedInt32Array()  # instance dans le MultiMesh de la tuile
var _ground_m: PackedFloat64Array = PackedFloat64Array()  # altitude de pose (m), NAN : à lire
var _color: PackedColorArray = PackedColorArray()
var _family: PackedStringArray = PackedStringArray()
## Modèles préparés : nom → index ; liste de {mesh, local: Transform3D, top, banner: Color}.
var _models: Dictionary = {}
var _model_list: Array[Dictionary] = []
## Tuiles : `MultiMeshInstance3D` par (modèle, tuile), et par type pour le fondu de portée.
var _tiles: Array[MultiMeshInstance3D] = []
var _tiles_by_kind: Array = []
var _kind_alpha: PackedFloat32Array = PackedFloat32Array()
var _landmarks: Dictionary = {}  # index de lieu → LandmarkModel
var _dirty: Dictionary = {}  # index de lieu → true (à reposer)
var _pose_scale := -1.0
var _pose_left := 0
var _pose_cursor := 0
var _shadows := true
var _shadow_distance := INF
var _year := -1


static func enabled() -> bool:
	return TownMaquetteData.enabled()


func setup(map_data: MapData, terrain: TerrainBuilder, layer: SettlementLayer) -> void:
	name = "TownMaquettes"
	var t0 := Time.get_ticks_msec()
	_map_data = map_data
	_terrain = terrain
	_layer = layer
	var entries: Array[Dictionary] = layer.data.settlements
	var count := entries.size()
	_radius.resize(count)
	_gain.resize(count)
	_top.resize(count)
	_range.resize(count)
	_yaw.resize(count)
	_model_scale.resize(count)
	_kind.resize(count)
	_model_of.resize(count)
	_model_of.fill(-1)
	_tile_of.resize(count)
	_tile_of.fill(-1)
	_slot.resize(count)
	_ground_m.resize(count)
	_ground_m.fill(NAN)
	_color.resize(count)
	_family.resize(count)
	_tiles_by_kind.clear()
	for k in TownMaquetteData.KINDS.size():
		_tiles_by_kind.append([])
	_kind_alpha.resize(TownMaquetteData.KINDS.size())
	_kind_alpha.fill(1.0)
	# 1. Tailles de base, villes emblématiques, réduction des voisins trop proches.
	var centers := PackedVector2Array()
	var fixed := PackedByteArray()
	centers.resize(count)
	fixed.resize(count)
	var landmark_scale := TownMaquetteData.landmark_scale()
	for i in count:
		var entry := entries[i]
		var id := str(entry["id"])
		var kind := TownMaquetteData.kind_of(entry)
		_kind[i] = TownMaquetteData.KINDS.find(kind)
		_range[i] = TownMaquetteData.visibility(kind)
		_gain[i] = TownMaquetteData.weight_gain(kind, float(entry.get("weight", 0)))
		_radius[i] = TownMaquetteData.width(kind) * 0.5 * _gain[i]
		_yaw[i] = TownMaquetteData.yaw_of(id)
		_family[i] = TownMaquetteData.family_of_province(str(entry.get("province", "")))
		centers[i] = layer.model_px(i)
		var plan := LandmarkLibrary.for_settlement(id)
		if not plan.is_empty():
			var landmark := LandmarkModel.create(plan, terrain, landmark_scale, true)
			if landmark != null:
				add_child(landmark)
				_landmarks[i] = landmark
				fixed[i] = 1
				_radius[i] = landmark.core_radius
				_top[i] = landmark.top_height()
				centers[i] = Vector2(landmark.position.x, landmark.position.z)
	var t1 := Time.get_ticks_msec()
	_factor = TownMaquetteData.solve_collisions(centers, _radius, fixed, TownMaquetteData.collision_min_scale(), TownMaquetteData.collision_margin())
	# 2. Modèle et tuile de chaque lieu du kit.
	var tile := TownMaquetteData.tile_size()
	var groups := {}  # "modèle|tuile" → indices de lieux
	var fallbacks := 0
	var reduced := 0
	var absorbed := 0
	var absorb_ratio := TownMaquetteData.collision_absorb_ratio()
	_absorbed.resize(count)
	_absorbed.fill(0)
	for i in count:
		if _landmarks.has(i):
			continue
		# Lieu noyé dans la maquette d'une ville emblématique (Vincennes dans Paris) : pas de maquette
		# propre, son nom et son écu restent.
		var swallowed := false
		for j: int in _landmarks:
			if centers[i].distance_to(centers[j]) < _radius[j] * absorb_ratio:
				swallowed = true
				break
		if swallowed:
			_absorbed[i] = 1
			absorbed += 1
			_radius[i] *= _factor[i]
			continue
		var kind: String = TownMaquetteData.KINDS[_kind[i]]
		var variant := TownMaquetteData.variant_of(str(entries[i]["id"]))
		var model_name := TownMaquetteData.model_name(kind, _family[i], variant)
		if model_name != TownMaquetteData.model_name_raw(kind, _family[i], variant):
			fallbacks += 1
		var m := _model_index(model_name)
		if m < 0:
			_radius[i] *= _factor[i]
			continue
		if _factor[i] < 0.999:
			reduced += 1
		_model_of[i] = m
		_model_scale[i] = TownMaquetteData.model_scale(kind) * _gain[i] * _factor[i]
		_radius[i] *= _factor[i]
		_top[i] = float(_model_list[m]["top"]) * _model_scale[i]
		_color[i] = _model_list[m]["banner"]
		var key := Vector3i(m, int(floor(centers[i].x / tile)), int(floor(centers[i].y / tile)))
		if not groups.has(key):
			groups[key] = PackedInt32Array()
		groups[key].append(i)
	# 3. Un MultiMesh par (modèle, tuile).
	var instances := 0
	var margin := TownMaquetteData.fade_margin()
	for key: Vector3i in groups:
		var members: PackedInt32Array = groups[key]
		var model: Dictionary = _model_list[key.x]
		var multimesh := MultiMesh.new()
		multimesh.transform_format = MultiMesh.TRANSFORM_3D
		multimesh.use_custom_data = true
		multimesh.mesh = model["mesh"]
		multimesh.instance_count = members.size()
		var mmi := MultiMeshInstance3D.new()
		mmi.name = "%s_%d_%d" % [model["name"], key.y, key.z]
		mmi.position = Vector3((float(key.y) + 0.5) * tile, 0.0, (float(key.z) + 0.5) * tile)
		mmi.multimesh = multimesh
		var kind_index := _kind[members[0]]
		# Portée de la tuile (culling) : celle du type, vue depuis n'importe quel point de la tuile ;
		# le fondu se fait par type sur la distance du rig (`update_view`).
		mmi.visibility_range_end = _range[members[0]] + margin + tile * 0.7072
		add_child(mmi)
		var t := _tiles.size()
		_tiles.append(mmi)
		(_tiles_by_kind[kind_index] as Array).append(mmi)
		for s in members.size():
			var i := members[s]
			_tile_of[i] = t
			_slot[i] = s
			_place(i)
			multimesh.set_instance_custom_data(s, _color[i].srgb_to_linear())
		instances += members.size()
	_pose_scale = MapData.vertical_scale()
	add_to_group(RenderQuality.CLIENT_GROUP)
	apply_render_quality(RenderQuality.preset())
	stats = {
		"places": count, "landmarks": _landmarks.size(), "instances": instances, "multimeshes": _tiles.size(),
		"models": _model_list.size(), "reduced": reduced, "absorbed": absorbed, "family_fallbacks": fallbacks,
		"setup_ms": Time.get_ticks_msec() - t0, "prepare_ms": t1 - t0,
	}
	print("TownMaquetteLayer: %s" % JSON.stringify(stats))


## Modèle préparé pour le MultiMesh (maillage aux matériaux partagés du kit, bannière teintée par
## instance), -1 s'il n'est pas importé.
func _model_index(model_name: String) -> int:
	if _models.has(model_name):
		return _models[model_name]
	var index := -1
	var scene := ModelLibrary.get_scene("settlements/" + model_name)
	var root: Node3D = null
	if scene != null:
		root = scene.instantiate() as Node3D
	if root != null:
		var found := root.find_children("*", "MeshInstance3D", true, false)
		if root is MeshInstance3D:
			found.push_front(root)
		if not found.is_empty():
			var mesh_instance := found[0] as MeshInstance3D
			# Repère du maillage dans le modèle (le kit décale son maillage sous le sol).
			var local := Transform3D.IDENTITY
			var node: Node3D = mesh_instance
			while node != null and node != root:
				local = node.transform * local
				node = node.get_parent() as Node3D
			var mesh := mesh_instance.mesh.duplicate() as Mesh
			var banner := Color(0.6, 0.6, 0.6)
			for surface in mesh.get_surface_count():
				var original := mesh.surface_get_material(surface)
				if original == null:
					continue
				var material_name := BuildingMaterials._base_name(original.resource_name)
				if material_name == "Banner":
					if original is BaseMaterial3D:
						banner = (original as BaseMaterial3D).albedo_color
					mesh.surface_set_material(surface, _banner_material())
					continue
				var shared := BuildingMaterials.material(material_name, "far")
				if shared != null:
					mesh.surface_set_material(surface, shared)
			var box := local * mesh.get_aabb()
			index = _model_list.size()
			_model_list.append({"name": model_name, "mesh": mesh, "local": local, "top": maxf(box.end.y, 0.0), "banner": banner})
		root.free()
	_models[model_name] = index
	return index


var _banner: ShaderMaterial


func _banner_material() -> ShaderMaterial:
	if _banner == null:
		_banner = ShaderMaterial.new()
		_banner.shader = BANNER_SHADER
	return _banner


## Altitude de pose (m) du lieu `i` : le sol au centre, abaissé à la moyenne de l'emprise si elle
## est plus basse (les bâtiments du kit se prolongent sous leur sol : mieux vaut enfoncer un bord
## que laisser flotter l'autre).
func _pose_m(i: int) -> float:
	var m := _ground_m[i]
	if is_nan(m):
		var px := _layer.model_px(i)
		m = _map_data.height_m_at(px.x, px.y)
		var r := _radius[i] * 0.6
		var mean := 0.0
		for offset: Vector2 in [Vector2(r, 0.0), Vector2(-r, 0.0), Vector2(0.0, r), Vector2(0.0, -r)]:
			mean += _map_data.height_m_at(px.x + offset.x, px.y + offset.y) * 0.25
		m = minf(m, mean)
		_ground_m[i] = m
	return m


## Écrit la transformation de l'instance du lieu `i` (position vraie, sol affiché, lacet, échelle).
func _place(i: int) -> void:
	var t := _tile_of[i]
	if t < 0:
		return
	var px := _layer.model_px(i)
	var y := maxf(MapData.display_height(_pose_m(i), px.x, px.y), 0.0)
	var mmi := _tiles[t]
	mmi.multimesh.set_instance_transform(_slot[i], Transform3D(Basis.IDENTITY, -mmi.position) * _world_transform(i, px, y))


## Transformation monde du maillage du lieu `i` posé en (`px`, `y`) : lacet, échelle, repère du
## maillage dans son modèle.
func _world_transform(i: int, px: Vector2, y: float) -> Transform3D:
	var basis := Basis(Vector3.UP, _yaw[i]).scaled(Vector3.ONE * _model_scale[i])
	var local: Transform3D = _model_list[_model_of[i]]["local"]
	return Transform3D(basis, Vector3(px.x, y, px.y)) * local


# --- Mises à jour ----------------------------------------------------------------------


## Bannières à la couleur du contrôleur (`color_of` : faction → Color) ; années des éléments
## datés des villes emblématiques. Écritures seulement quand la couleur change.
func refresh(sim: Object, color_of: Callable) -> void:
	if _layer == null or _layer.data == null:
		return
	if sim != null and sim.has_method("get_date_label"):
		var year := LandmarkModel.year_of(str(sim.call("get_date_label")))
		if year > 0 and year != _year:
			_year = year
			for landmark: LandmarkModel in _landmarks.values():
				landmark.set_year(year)
	if not color_of.is_valid():
		return
	var entries: Array[Dictionary] = _layer.data.settlements
	var colors := {}  # faction → Color
	for i in entries.size():
		var t := _tile_of[i]
		if t < 0:
			continue
		var faction := str(entries[i].get("controller", ""))
		if not colors.has(faction):
			var value: Variant = color_of.call(faction) if faction != "" else null
			colors[faction] = value if value is Color else _model_list[_model_of[i]]["banner"]
		var color: Color = colors[faction]
		if color != _color[i]:
			_color[i] = color
			_tiles[t].multimesh.set_instance_custom_data(_slot[i], color.srgb_to_linear())


## Couleur de bannière affichée pour le lieu `i` (tests).
func banner_color(i: int) -> Color:
	return _color[i] if i >= 0 and i < _color.size() else Color.BLACK


## Lieux d'un morceau de terrain recalé : reposés à la prochaine image.
func reground(indices: PackedInt32Array) -> void:
	for i in indices:
		_dirty[i] = true


## Ancrages fins changés (`SettlementLayer.apply_fine_anchors`) : tout est relu et reposé.
func reposition_all() -> void:
	_ground_m.fill(NAN)
	for i in _tile_of.size():
		_place(i)
	_dirty.clear()
	_pose_left = 0


func update_view(camera_distance: float) -> void:
	# Fondu de portée par type, sur la distance du rig.
	var margin := maxf(TownMaquetteData.fade_margin(), 0.001)
	for k in _tiles_by_kind.size():
		var limit := TownMaquetteData.visibility(TownMaquetteData.KINDS[k])
		var alpha := clampf((limit - camera_distance) / margin, 0.0, 1.0)
		if alpha != _kind_alpha[k]:
			_kind_alpha[k] = alpha
			for mmi: MultiMeshInstance3D in _tiles_by_kind[k]:
				mmi.visible = alpha > 0.0
				mmi.transparency = 1.0 - alpha
	var shadows := camera_distance < _shadow_distance
	if shadows != _shadows:
		_shadows = shadows
		var mode := GeometryInstance3D.SHADOW_CASTING_SETTING_ON if shadows else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		for mmi in _tiles:
			mmi.cast_shadow = mode
	# Sol : morceaux recalés, puis tour complet par tranches quand l'échelle verticale change.
	if not _dirty.is_empty():
		for i: int in _dirty:
			_place(i)
		_dirty.clear()
	if MapData.vertical_scale() != _pose_scale:
		_pose_scale = MapData.vertical_scale()
		_pose_left = _tile_of.size()
	if _pose_left > 0:
		_pose_slice(FrameBudget.in_frame())


func _pose_slice(sliced: bool) -> void:
	var count := _tile_of.size()
	var done := 0
	var i := _pose_cursor % maxi(count, 1)
	while _pose_left > 0 and (not sliced or done < POSE_SLICE):
		_place(i)
		done += 1
		_pose_left -= 1
		i = (i + 1) % count
	_pose_cursor = i


## Termine tout de suite les poses en attente et les cuissons des villes emblématiques
## (captures, tests).
func flush() -> void:
	for i: int in _dirty:
		_place(i)
	_dirty.clear()
	if _pose_left > 0:
		_pose_slice(false)
	for landmark: LandmarkModel in _landmarks.values():
		landmark.flush_bake()


## FC1 : ombres portées des maquettes jusqu'à `model_shadow_distance` (préréglage de qualité).
func apply_render_quality(preset: Dictionary) -> void:
	_shadow_distance = float(preset.get("model_shadow_distance", INF))


# --- Accès (emprises de `SettlementLayer`, tests) ---------------------------------------


## Demi-largeur (unités monde) de la maquette du lieu `i`, réduction de collision comprise.
func radius_of(i: int) -> float:
	return _radius[i] if i >= 0 and i < _radius.size() else 0.0


## Hauteur (unités monde) de la maquette du lieu `i` au-dessus de son sol.
func top_of(i: int) -> float:
	return _top[i] if i >= 0 and i < _top.size() else 0.0


## Réduction de collision du lieu `i` (1 = taille de base).
## Vrai si le lieu `i` est noyé dans la maquette d'une ville emblématique (pas de maquette propre).
func is_absorbed(i: int) -> bool:
	return i >= 0 and i < _absorbed.size() and _absorbed[i] == 1


## Gain de taille du lieu `i` selon son poids (`TownMaquetteData.weight_gain`).
func gain_of(i: int) -> float:
	return _gain[i] if i >= 0 and i < _gain.size() else 1.0


func factor_of(i: int) -> float:
	return _factor[i] if i >= 0 and i < _factor.size() else 1.0


## Portée (distance du rig) de la maquette du lieu `i` ; INF pour une ville emblématique.
func range_of(i: int) -> float:
	if _landmarks.has(i):
		return INF
	return _range[i] if i >= 0 and i < _range.size() else 0.0


func family_of(i: int) -> String:
	return _family[i] if i >= 0 and i < _family.size() else ""


func is_landmark(i: int) -> bool:
	return _landmarks.has(i)


func landmark_model(i: int) -> LandmarkModel:
	return _landmarks.get(i)


## Hauteur monde du sol du modèle du lieu `i` (tests) ; 0 pour une ville emblématique.
func pose_height(i: int) -> float:
	if i < 0 or i >= _tile_of.size() or _tile_of[i] < 0:
		return 0.0
	var px := _layer.model_px(i)
	return maxf(MapData.display_height(_pose_m(i), px.x, px.y), 0.0)


## Nombre d'instances de MultiMesh (lieux du kit).
func instance_count() -> int:
	var total := 0
	for mmi in _tiles:
		total += mmi.multimesh.instance_count
	return total


## Transformation monde de l'instance du lieu `i` (tests ; recalculée : le rendu factice de
## `--headless` ne relit pas un MultiMesh) ; identité pour une ville emblématique.
## Transformation locale du maillage dans son modèle (les kits recentrés portent un décalage de
## nœud) ; identité pour une ville emblématique ou un lieu sans modèle.
func model_local(i: int) -> Transform3D:
	if i < 0 or i >= _model_of.size() or _landmarks.has(i) or _model_of[i] < 0:
		return Transform3D.IDENTITY
	return _model_list[_model_of[i]]["local"]


func instance_transform(i: int) -> Transform3D:
	if i < 0 or i >= _tile_of.size() or _tile_of[i] < 0:
		return Transform3D.IDENTITY
	return _world_transform(i, _layer.model_px(i), pose_height(i))
