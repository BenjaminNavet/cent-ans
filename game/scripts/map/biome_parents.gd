class_name BiomeParents
extends RefCounted

## Chantier TX 2a (ADR 0238) : les 15 classes de `data/map/biomes.png` (0 mer, 1-7 base, 8-14
## sous-classes régionales) et leur repli sur le `parent`. La table vient de
## `data/map/biome_parents.json`, écrite par `cent-ans geo biomes` depuis `data/map/biomes.yaml`
## (aucune copie à la main). Rendu seulement, aucune règle de jeu.

const FILE := "map/biome_parents.json"
const COUNT := 15
## Première classe régionale (parent obligatoire).
const FIRST_REGIONAL := 8
## Lignes 8..15 de la table du sol : paysages agricoles ME8 ; les biomes 8..14 sont rangés
## à partir de cette ligne (`table_row`).
const REGIONAL_ROW_BASE := 16

static var _parents: PackedInt32Array = PackedInt32Array()


## Table des parents (`COUNT` entrées, `parents()[i]` = `i` pour une classe de base). Sans fichier,
## chaque classe est son propre parent (les sous-classes ne se replient alors sur rien).
static func parents() -> PackedInt32Array:
	if _parents.is_empty():
		_parents = parse(DataFile.read_json(FILE))
	return _parents


## Table depuis le document JSON (testable sans fichier).
static func parse(doc: Variant) -> PackedInt32Array:
	var table := PackedInt32Array()
	table.resize(COUNT)
	for i in COUNT:
		table[i] = i
	if doc is Dictionary:
		var list: Array = (doc as Dictionary).get("parents", [])
		for i in mini(list.size(), COUNT):
			table[i] = clampi(int(list[i]), 0, COUNT - 1)
	return table


static func reset() -> void:
	_parents = PackedInt32Array()


## Parent de `biome` (lui-même pour une classe de base).
static func parent_of(biome: int, table: PackedInt32Array = PackedInt32Array()) -> int:
	var t := table if not table.is_empty() else parents()
	return t[clampi(biome, 0, COUNT - 1)]


## Première classe de la chaîne des parents dont `known` (Callable int -> bool) est vrai ;
## la classe de base atteinte si aucune ne l'est.
static func resolve(biome: int, known: Callable, table: PackedInt32Array = PackedInt32Array()) -> int:
	var t := table if not table.is_empty() else parents()
	var b := clampi(biome, 0, COUNT - 1)
	for _i in COUNT:
		if known.call(b) or t[b] == b:
			return b
		b = t[b]
	return b


## Ligne de la table du sol (`HbGround`) du biome : 1..7 pour la base, 16.. pour 8..14.
static func table_row(biome: int) -> int:
	return biome if biome < FIRST_REGIONAL else REGIONAL_ROW_BASE + biome - FIRST_REGIONAL



## Vrai si `mask` (bit `b` = biome `b` admis) admet `biome` ou l'un de ses ancêtres : une classe
## régionale que la donnée ne cite pas hérite de la décision de son parent.
static func mask_allows(mask: int, biome: int, table: PackedInt32Array = PackedInt32Array()) -> bool:
	var t := table if not table.is_empty() else parents()
	var b := clampi(biome, 0, COUNT - 1)
	for _i in 3:
		if (mask >> b) & 1 == 1:
			return true
		if t[b] == b:
			return false
		b = t[b]
	return false
