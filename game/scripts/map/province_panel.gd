class_name ProvincePanel
extends PanelContainer

## Panneau latéral parchemin : nom, propriétaire, capitale, terrain de la province
## sélectionnée. Données issues de provinces.geojson pour l'instant (M2 : `core/`).

const TERRAIN_LABELS := {
	"plains": "Plaines", "hills": "Collines", "mountains": "Montagnes",
	"forest": "Forêt", "marsh": "Marais", "coast": "Littoral", "highlands": "Hautes terres",
}

@onready var name_label: Label = %NameLabel
@onready var owner_value: Label = %OwnerValue
@onready var capital_value: Label = %CapitalValue
@onready var terrain_value: Label = %TerrainValue
@onready var id_value: Label = %IdValue


func show_province(province: Dictionary) -> void:
	if province.is_empty():
		hide()
		return
	name_label.text = province.get("name", "?")
	var owner: String = province.get("owner", "")
	owner_value.text = owner if owner != "" else "—"
	var capital: String = province.get("capital_name", "")
	capital_value.text = capital if capital != "" else "—"
	var terrain: String = province.get("terrain", "")
	terrain_value.text = TERRAIN_LABELS.get(terrain, terrain if terrain != "" else "—")
	id_value.text = "%s (index %d)" % [province.get("id", "?"), province.get("index", 0)]
	show()
