class_name TownMaquetteLayer
extends Node3D

## Lot GC2 (ADR 0158) : villes stylisées de la carte de campagne. Chaque lieu est une maquette du
## kit (`assets/models/settlements/<type>[_<famille>]_<a|b>.glb`) posée à sa vraie position, à
## taille monde constante par type (`data/art/town_maquettes.json`, voir `TownMaquetteData`) :
## - un `MultiMesh` par (modèle, tuile de carte de `tile_size` unités) : culling par tuile, portée
##   par type (fondu sur `fade_margin` selon la distance du rig) ;
## - une seule surface par modèle des six familles (matériau `Kit`, teintes cuites en couleur de
##   sommet, lot GC6-perf) et un seul `ShaderMaterial` (`maquette_kit.gdshader`) pour tous : un
##   appel de dessin par MultiMesh ; l'ancien kit (repli) garde ses surfaces par matière ;
## - famille d'architecture par province (culture, région, religion), repli sur l'Ouest tant que
##   le modèle de la famille manque ; variante et lacet tirés par id de lieu ;
## - un lieu dont le disque touche celui d'un lieu plus important est réduit (`collision`) ;
## - bannières (alpha nul de la couleur de sommet, ou matériau `Banner` de l'ancien kit) à la
##   couleur du contrôleur, par donnée d'instance ;
## - ombres portées coupées par type au-delà de `shadow_range` (petits lieux) ;
## - posé au sol affiché (hauteur en mètres lue une fois, remise à l'échelle verticale courante
##   par morceau recalé puis par tranches) ;
## - les villes emblématiques (`data/landmarks/`) gardent leur `LandmarkModel`, grossi de
##   `landmark_scale`.
## - lot DN camp-bati (D1) : si `data/art/dn_campaign_models.json` donne un glb généré pour le type
##   et la famille d'un lieu, ce glb (mono-surface texturé, éclairage standard) remplace la maquette
##   sous `near_distance` (fondu croisé sur `fade_margin`) ; la maquette stylisée garde le lointain.
##   Un MultiMesh par (modèle généré, tuile) ; la couleur du contrôleur ne teinte qu'une bannière
##   procédurale (hampe + toile, `maquette_banner.gdshader`) posée au sommet. Table vide : rien
##   ne change.
## Style par défaut (`map.town_style`), `--town-style=real` rend les villes 1:1. Purement visuel.

const BANNER_SHADER := preload("res://shaders/maquette_banner.gdshader")
const KIT_SHADER := preload("res://shaders/maquette_kit.gdshader")
## Nom du matériau de la surface unique des modèles des six familles (`settlements_east.bake_kit`).
const KIT_MATERIAL := "Kit"
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
var _dn_scale: PackedFloat32Array = PackedFloat32Array()  # échelle du glb généré (0 : lieu sans glb généré)
var _dn_model_of: PackedInt32Array = PackedInt32Array()  # index dans `_dn_list`, -1 sinon
var _color: PackedColorArray = PackedColorArray()
var _family: PackedStringArray = PackedStringArray()
var _subfamily: PackedStringArray = PackedStringArray()  # sous-famille des glb générés (DN-MAQ), "" si aucune
## Modèles préparés : nom → index ; liste de {mesh, local: Transform3D, top, banner: Color}.
var _models: Dictionary = {}
var _model_list: Array[Dictionary] = []
## Tuiles : `MultiMeshInstance3D` par (modèle, tuile), et par type pour le fondu de portée.
var _tiles: Array[MultiMeshInstance3D] = []
## Parallèles à `_tiles` (null : tuile sans glb généré) : glb généré, bannière, entrée de `_dn_list`.
var _dn_tiles: Array[MultiMeshInstance3D] = []
var _dn_banners: Array[MultiMeshInstance3D] = []
var _dn_tile_model: PackedInt32Array = PackedInt32Array()
var _tile_kind: PackedInt32Array = PackedInt32Array()  # type de lieu de chaque tuile
## Glb générés préparés : nom → index ; liste de {mesh, local, top, width, entry}.
var _dn_models: Dictionary = {}
var _dn_list: Array[Dictionary] = []
var _dn_banner_mesh: ArrayMesh
var _dn_banner_material: ShaderMaterial
var _dn_alpha: PackedFloat32Array = PackedFloat32Array()  # par tuile : part de glb généré (0 à 1)
var _tiles_by_kind: Array = []  # toutes les tuiles du type (ombres)
var _fade_by_kind: Array = []  # tuiles du type sans glb généré (fondu de portée)
var _kind_alpha: PackedFloat32Array = PackedFloat32Array()
var _kind_shadows: PackedByteArray = PackedByteArray()  # 1 : les tuiles du type portent une ombre
var _kind_shadow_range: PackedFloat32Array = PackedFloat32Array()  # `shadow_range` du type (INF : sans limite)
var _landmarks: Dictionary = {}  # index de lieu → LandmarkModel
var _dirty: Dictionary = {}  # index de lieu → true (à reposer)
var _pose_scale := -1.0
var _pose_left := 0
var _pose_cursor := 0
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
	_dn_scale.resize(count)
	_dn_model_of.resize(count)
	_dn_model_of.fill(-1)
	_family.resize(count)
	_subfamily.resize(count)
	_tiles_by_kind.clear()
	_fade_by_kind.clear()
	for k in TownMaquetteData.KINDS.size():
		_tiles_by_kind.append([])
		_fade_by_kind.append([])
	_kind_alpha.resize(TownMaquetteData.KINDS.size())
	_kind_alpha.fill(1.0)
	_kind_shadows.resize(TownMaquetteData.KINDS.size())
	_kind_shadows.fill(1)
	_kind_shadow_range.resize(TownMaquetteData.KINDS.size())
	for k in TownMaquetteData.KINDS.size():
		_kind_shadow_range[k] = TownMaquetteData.shadow_range(TownMaquetteData.KINDS[k])
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
		_subfamily[i] = TownMaquetteData.subfamily_of_province(str(entry.get("province", "")))
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
		# Glb généré (D1) : le groupe porte l'index du glb (w), -1 sans.
		var dn := _dn_for(i, kind)
		if dn >= 0:
			_dn_model_of[i] = dn
			_dn_scale[i] = _dn_scale_of(dn, kind, i)
		var key := Vector4i(m, int(floor(centers[i].x / tile)), int(floor(centers[i].y / tile)), dn)
		if not groups.has(key):
			groups[key] = PackedInt32Array()
		groups[key].append(i)
	# 3. Un MultiMesh par (modèle, tuile).
	var instances := 0
	var margin := TownMaquetteData.fade_margin()
	for key: Vector4i in groups:
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
		_tile_kind.append(kind_index)
		(_tiles_by_kind[kind_index] as Array).append(mmi)
		if key.w < 0:
			(_fade_by_kind[kind_index] as Array).append(mmi)
			_dn_tiles.append(null)
			_dn_banners.append(null)
			_dn_tile_model.append(-1)
			_dn_alpha.append(0.0)
		else:
			_add_dn_tiles(key.w, mmi, members.size(), kind_index)
		for s in members.size():
			var i := members[s]
			_tile_of[i] = t
			_slot[i] = s
			_place(i)
			multimesh.set_instance_custom_data(s, _color[i].srgb_to_linear())
			var banner_mmi := _dn_banners[t]
			if banner_mmi != null:
				banner_mmi.multimesh.set_instance_custom_data(s, _color[i].srgb_to_linear())
		instances += members.size()
	_pose_scale = MapData.vertical_scale()
	add_to_group(RenderQuality.CLIENT_GROUP)
	apply_render_quality(RenderQuality.preset())
	stats = {
		"places": count, "landmarks": _landmarks.size(), "instances": instances, "multimeshes": _tiles.size(),
		"models": _model_list.size(), "dn_models": _dn_list.size(), "dn_places": _dn_places(), "reduced": reduced, "absorbed": absorbed, "family_fallbacks": fallbacks,
		"setup_ms": Time.get_ticks_msec() - t0, "prepare_ms": t1 - t0,
	}
	print("TownMaquetteLayer: %s" % JSON.stringify(stats))


## Modèle préparé pour le MultiMesh, -1 s'il n'est pas importé. Surface `Kit` (six familles) :
## le matériau unique des maquettes, bannière teintée par instance dans le shader. Ancien kit :
## matériaux partagés de `BuildingMaterials`, surface `Banner` à part.
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
				if material_name == KIT_MATERIAL:
					banner = _kit_banner_color(mesh, surface, banner)
					mesh.surface_set_material(surface, _kit_material())
					continue
				if material_name == "Banner":
					if original is BaseMaterial3D:
						banner = (original as BaseMaterial3D).albedo_color
					_note_banner_size(mesh, surface)
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
var _banner_size := 0.0
var _kit: ShaderMaterial


## Matériau unique des surfaces `Kit`, partagé par tous les modèles et toutes les tuiles.
func _kit_material() -> ShaderMaterial:
	if _kit == null:
		_kit = ShaderMaterial.new()
		_kit.shader = KIT_SHADER
		_kit.resource_name = KIT_MATERIAL
	return _kit


## Couleur de bannière par défaut d'une surface `Kit` : celle cuite par Blender sur les faces
## marquées (alpha nul), linéaire dans le maillage, rendue en sRGB comme les couleurs de faction.
func _kit_banner_color(mesh: Mesh, surface: int, fallback: Color) -> Color:
	var colors: Variant = mesh.surface_get_arrays(surface)[Mesh.ARRAY_COLOR]
	if colors is PackedColorArray:
		for color: Color in colors:
			if color.a < 0.5:
				return Color(color.r, color.g, color.b).linear_to_srgb()
	return fallback


func _banner_material() -> ShaderMaterial:
	if _banner == null:
		_banner = ShaderMaterial.new()
		_banner.shader = BANNER_SHADER
		# AS5 : onde de vent (data/fx/map_fire_wind.json) ; amplitude nulle si éteinte.
		var wind := MapFireWind.section("maquette_banner")
		_banner.set_shader_parameter("amplitude", float(wind.get("amplitude", 0.0)))
		for key in ["wave_speed", "wave_length", "gust_gain", "calm"]:
			if wind.has(key):
				_banner.set_shader_parameter(key, float(wind[key]))
	return _banner


## AS5 : plus grande dimension d'une toile de bannière des modèles préparés (mètres du modèle),
## base de l'amplitude de l'onde de vent.
func _note_banner_size(mesh: Mesh, surface: int) -> void:
	var vertices: Variant = mesh.surface_get_arrays(surface)[Mesh.ARRAY_VERTEX]
	if not vertices is PackedVector3Array or (vertices as PackedVector3Array).is_empty():
		return
	var box := AABB((vertices as PackedVector3Array)[0], Vector3.ZERO)
	for v: Vector3 in vertices:
		box = box.expand(v)
	_banner_size = maxf(_banner_size, box.get_longest_axis_size())
	_banner_material().set_shader_parameter("banner_size", _banner_size)


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
	if _dn_model_of[i] >= 0 and _dn_tiles[t] != null:
		var offset := Transform3D(Basis.IDENTITY, -mmi.position)
		_dn_tiles[t].multimesh.set_instance_transform(_slot[i], offset * dn_world_transform(i, px, y))
		var banner_mmi := _dn_banners[t]
		if banner_mmi != null:
			banner_mmi.multimesh.set_instance_transform(_slot[i], offset * _dn_banner_transform(i, px, y))


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
			if _dn_banners[t] != null:
				_dn_banners[t].multimesh.set_instance_custom_data(_slot[i], color.srgb_to_linear())


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
	for k in _fade_by_kind.size():
		var limit := TownMaquetteData.visibility(TownMaquetteData.KINDS[k])
		var alpha := clampf((limit - camera_distance) / margin, 0.0, 1.0)
		if alpha != _kind_alpha[k]:
			_kind_alpha[k] = alpha
			for mmi: MultiMeshInstance3D in _fade_by_kind[k]:
				mmi.visible = alpha > 0.0
				mmi.transparency = 1.0 - alpha
	_update_dn_view(camera_distance)
	# Ombres portées, par type : jusqu'à `model_shadow_distance` (FC1) et, pour les petits lieux,
	# jusqu'à leur `shadow_range` (une passe d'ombre en moins par tuile au-delà).
	for k in _tiles_by_kind.size():
		var shadows := camera_distance < minf(_shadow_distance, _kind_shadow_range[k])
		if shadows != (_kind_shadows[k] == 1):
			_kind_shadows[k] = 1 if shadows else 0
			var mode := GeometryInstance3D.SHADOW_CASTING_SETTING_ON if shadows else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
			for mmi: MultiMeshInstance3D in _tiles_by_kind[k]:
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


## Vrai si les tuiles du type `kind` portent une ombre (tests).
func kind_casts_shadow(kind: String) -> bool:
	var k := TownMaquetteData.KINDS.find(kind)
	return k >= 0 and k < _kind_shadows.size() and _kind_shadows[k] == 1


## Nombre de surfaces du modèle du lieu `i` et matériau de la première (tests) ; 0 et null pour
## une ville emblématique ou un lieu sans modèle.
func model_surface_count(i: int) -> int:
	if i < 0 or i >= _model_of.size() or _model_of[i] < 0:
		return 0
	return (_model_list[_model_of[i]]["mesh"] as Mesh).get_surface_count()


func model_material(i: int) -> Material:
	if i < 0 or i >= _model_of.size() or _model_of[i] < 0:
		return null
	return (_model_list[_model_of[i]]["mesh"] as Mesh).surface_get_material(0)


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


# --- Glb générés (DN camp-bati, D1) ---------------------------------------------------


## Index du glb généré du lieu `i` (type `kind`), -1 si la table n'en donne pas ou si le fichier
## manque (la maquette reste alors seule).
func _dn_for(i: int, kind: String) -> int:
	if DnCampaignModels.is_empty():
		return -1
	var variant_index := absi(str(_layer.data.settlements[i]["id"]).hash())
	var entry := DnCampaignModels.entry_for(kind, _family[i], variant_index, _subfamily[i])
	if entry.is_empty():
		return -1
	return _dn_index(entry)


## Glb généré préparé (premier maillage, repère local, largeur native mesurée si absente) ; -1 s'il
## n'est pas importé. Le matériau importé (albédo baké, éclairage standard) est gardé tel quel.
func _dn_index(entry: Dictionary) -> int:
	var path := str(entry.get("path", ""))
	if _dn_models.has(path):
		return _dn_models[path]
	var index := -1
	var scene := ModelLibrary.get_scene(DnCampaignModels.model_name(entry))
	var root: Node3D = null
	if scene != null:
		root = scene.instantiate() as Node3D
	if root != null:
		var found := root.find_children("*", "MeshInstance3D", true, false)
		if root is MeshInstance3D:
			found.push_front(root)
		if not found.is_empty():
			var mesh_instance := found[0] as MeshInstance3D
			var local := Transform3D.IDENTITY
			var node: Node3D = mesh_instance
			while node != null and node != root:
				local = node.transform * local
				node = node.get_parent() as Node3D
			var mesh := mesh_instance.mesh.duplicate() as Mesh
			DnCampaignModels.brighten(mesh, entry)
			var box := local * mesh.get_aabb()
			var width := float(entry.get("native_width", maxf(box.size.x, box.size.z)))
			index = _dn_list.size()
			_dn_list.append({"path": path, "mesh": mesh, "local": local, "top": maxf(box.end.y, 0.0), "width": maxf(width, 0.001), "entry": entry})
		root.free()
	_dn_models[path] = index
	return index


## Échelle du glb généré `dn` pour le lieu `i` : même largeur monde que la maquette du type.
func _dn_scale_of(dn: int, kind: String, i: int) -> float:
	var info: Dictionary = _dn_list[dn]
	var entry: Dictionary = info["entry"]
	return TownMaquetteData.width(kind) * _gain[i] * _factor[i] * float(entry.get("scale", 1.0)) / float(info["width"])


func dn_world_transform(i: int, px: Vector2, y: float) -> Transform3D:
	var basis := Basis(Vector3.UP, _yaw[i]).scaled(Vector3.ONE * _dn_scale[i])
	var local: Transform3D = _dn_list[_dn_model_of[i]]["local"]
	return Transform3D(basis, Vector3(px.x, y, px.y)) * local


## Bannière procédurale au sommet du glb généré : hauteur de hampe = part de la hauteur du modèle.
func _dn_banner_transform(i: int, px: Vector2, y: float) -> Transform3D:
	var info: Dictionary = _dn_list[_dn_model_of[i]]
	var top: float = float(info["top"]) * _dn_scale[i]
	var k := maxf(top * DnCampaignModels.banner_pole_ratio(), 0.001)
	var basis := Basis(Vector3.UP, _yaw[i]).scaled(Vector3.ONE * k)
	return Transform3D(basis, Vector3(px.x, y + top, px.y))


## Crée le MultiMesh du glb `dn` (et sa bannière) en parallèle de la tuile de maquette `mmi`
## (mêmes emplacements) ; la tuile de maquette se fond avec lui (`_update_dn_view`).
func _add_dn_tiles(dn: int, mmi: MultiMeshInstance3D, member_count: int, kind_index: int) -> void:
	var info: Dictionary = _dn_list[dn]
	var multimesh := MultiMesh.new()
	multimesh.transform_format = MultiMesh.TRANSFORM_3D
	multimesh.mesh = info["mesh"]
	multimesh.instance_count = member_count
	var dn_mmi := MultiMeshInstance3D.new()
	dn_mmi.name = "dn_%s" % mmi.name
	dn_mmi.position = mmi.position
	dn_mmi.multimesh = multimesh
	var near := DnCampaignModels.near_distance(info["entry"])
	dn_mmi.visibility_range_end = near + DnCampaignModels.fade_margin() + TownMaquetteData.tile_size() * 0.7072
	dn_mmi.visible = false
	add_child(dn_mmi)
	(_tiles_by_kind[kind_index] as Array).append(dn_mmi)
	var banner_mmi: MultiMeshInstance3D = null
	if bool((info["entry"] as Dictionary).get("banner", true)):
		var banner_mm := MultiMesh.new()
		banner_mm.transform_format = MultiMesh.TRANSFORM_3D
		banner_mm.use_custom_data = true
		banner_mm.mesh = _banner_mesh()
		banner_mm.instance_count = member_count
		banner_mmi = MultiMeshInstance3D.new()
		banner_mmi.name = "dn_banner_%s" % mmi.name
		banner_mmi.position = mmi.position
		banner_mmi.multimesh = banner_mm
		banner_mmi.material_override = _dn_banner_mat()
		banner_mmi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		banner_mmi.visibility_range_end = dn_mmi.visibility_range_end
		banner_mmi.visible = false
		add_child(banner_mmi)
	_dn_tiles.append(dn_mmi)
	_dn_banners.append(banner_mmi)
	_dn_tile_model.append(dn)
	_dn_alpha.append(0.0)


## Bannière unitaire : hampe (hauteur 1) et toile 0,55 × 0,38 sur une grille 6 × 3 (onde de vent).
func _banner_mesh() -> ArrayMesh:
	if _dn_banner_mesh != null:
		return _dn_banner_mesh
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var pole := 0.02
	var corners := [Vector3(-pole, 0, -pole), Vector3(pole, 0, -pole), Vector3(pole, 0, pole), Vector3(-pole, 0, pole)]
	for c in 4:
		var a: Vector3 = corners[c]
		var b: Vector3 = corners[(c + 1) % 4]
		var up := Vector3(0, 1.0, 0)
		for v: Vector3 in [a, b, b + up, a, b + up, a + up]:
			st.set_normal((a + b).normalized())
			st.add_vertex(v)
	var cols := 6
	var rows := 3
	var width := 0.55
	var height := 0.38
	for r in rows:
		for c in cols:
			var x0 := width * c / cols
			var x1 := width * (c + 1) / cols
			var y0 := 1.0 - height * r / rows
			var y1 := 1.0 - height * (r + 1) / rows
			for v: Vector3 in [Vector3(x0, y0, 0), Vector3(x1, y0, 0), Vector3(x1, y1, 0), Vector3(x0, y0, 0), Vector3(x1, y1, 0), Vector3(x0, y1, 0)]:
				st.set_normal(Vector3(0, 0, 1))
				st.add_vertex(v)
	_dn_banner_mesh = st.commit()
	return _dn_banner_mesh


func _dn_banner_mat() -> ShaderMaterial:
	if _dn_banner_material == null:
		_dn_banner_material = _banner_material().duplicate() as ShaderMaterial
		_dn_banner_material.set_shader_parameter("banner_size", 0.55)
	return _dn_banner_material


## Fondu croisé maquette / glb généré par tuile : le glb apparaît sous `near + marge` (opacité
## pleine à `near`), la maquette de la tuile reste dessous jusqu'à ce que le glb soit opaque,
## puis disparaît ; au-delà de la portée du type, rien.
func _update_dn_view(camera_distance: float) -> void:
	if _dn_list.is_empty():
		return
	var margin := maxf(DnCampaignModels.fade_margin(), 0.001)
	var fade := maxf(TownMaquetteData.fade_margin(), 0.001)
	for t in _tiles.size():
		var dn_mmi := _dn_tiles[t]
		if dn_mmi == null:
			continue
		var info: Dictionary = _dn_list[_dn_tile_model[t]]
		var near := DnCampaignModels.near_distance(info["entry"])
		var limit := TownMaquetteData.visibility(TownMaquetteData.KINDS[_tile_kind[t]])
		var dn_alpha := clampf((near + margin - camera_distance) / margin, 0.0, 1.0) if camera_distance < limit else 0.0
		if dn_alpha != _dn_alpha[t]:
			_dn_alpha[t] = dn_alpha
			dn_mmi.visible = dn_alpha > 0.0
			dn_mmi.transparency = 1.0 - dn_alpha
			if _dn_banners[t] != null:
				_dn_banners[t].visible = dn_alpha > 0.0
				_dn_banners[t].transparency = 1.0 - dn_alpha
		var gen := _tiles[t]
		var gen_alpha := clampf((limit - camera_distance) / fade, 0.0, 1.0)
		gen.visible = dn_alpha < 1.0 and gen_alpha > 0.0
		gen.transparency = 1.0 - gen_alpha


## Nombre de lieux affichés par un glb généré.
func _dn_places() -> int:
	var total := 0
	for dn in _dn_model_of:
		if dn >= 0:
			total += 1
	return total


## Accès tests : glb généré du lieu `i` (-1 sans), part de glb généré de sa tuile (0 à 1),
## visibilité de la maquette de sa tuile.
func dn_model_of(i: int) -> int:
	return _dn_model_of[i] if i >= 0 and i < _dn_model_of.size() else -1


func dn_alpha_of(i: int) -> float:
	if i < 0 or i >= _tile_of.size() or _tile_of[i] < 0:
		return 0.0
	return _dn_alpha[_tile_of[i]]


func maquette_visible(i: int) -> bool:
	return i >= 0 and i < _tile_of.size() and _tile_of[i] >= 0 and _tiles[_tile_of[i]].visible


func dn_instance_count() -> int:
	var total := 0
	for mmi in _dn_tiles:
		if mmi != null:
			total += mmi.multimesh.instance_count
	return total


func dn_banner_count() -> int:
	var total := 0
	for mmi in _dn_banners:
		if mmi != null:
			total += mmi.multimesh.instance_count
	return total
