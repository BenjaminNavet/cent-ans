class_name ClassesSection
extends ProvinceSection

## Onglet « Ville » du panneau de province : une ligne par classe de population (paysans,
## bourgeois, clergé, noblesse) avec effectif et quatre jauges (mécontentement, santé, richesse,
## biens). `data["classes"]` : `get_province_city` de la simulation ; vide : message de repli.

const CLASS_LABELS := {"peasants": "Paysans", "burghers": "Bourgeois", "clergy": "Clergé", "nobility": "Noblesse"}
## Jauge → (nom court, vrai si une valeur haute est mauvaise : mécontentement).
const GAUGE_SPECS := [
	["unrest", "Mécont.", true],
	["health", "Santé", false],
	["wealth", "Richesse", false],
	["goods_satisfaction", "Biens", false],
]
## F2 : taille des icônes des lignes.
const ROW_ICON := 20.0


func _init() -> void:
	super("ClassesList", 3)


func _render(data: Dictionary) -> void:
	UiBuild.clear_children(self)
	var classes: Dictionary = data.get("classes", {})
	if classes.is_empty():
		var label := Label.new()
		label.text = "Données de ville indisponibles."
		add_child(label)
		return
	for class_id in ["peasants", "burghers", "clergy", "nobility"]:
		if not classes.has(class_id):
			continue
		add_child(_make_class_row(class_id, classes[class_id]))


func _make_class_row(class_id: String, data: Dictionary) -> Control:
	# Q6 : ligne à retour (jauges sous le nom quand la zone `SIDE_PANEL` est étroite) ; une
	# ligne fixe de 420 px élargissait le panneau hors de l'écran en vue 1280×720.
	var row := HFlowContainer.new()
	row.add_theme_constant_override("h_separation", 8)
	var name_chip := IconChip.create("class_" + class_id, str(CLASS_LABELS.get(class_id, class_id)), RichTooltip.population_class(class_id, data), ROW_ICON, 14)
	name_chip.custom_minimum_size = Vector2(104, 0)
	row.add_child(name_chip)
	var count_label := Label.new()
	count_label.text = Money.digits(int(data.get("count", 0)))
	count_label.custom_minimum_size = Vector2(62, 0)
	row.add_child(count_label)
	for spec in GAUGE_SPECS:
		var key: String = spec[0]
		var invert: bool = spec[2]
		var value := float(data.get(key, 0))
		row.add_child(_make_gauge(value, invert, spec[1], key))
	return row


## Petite jauge colorée (fond gris, remplissage vert → rouge selon `invert`), icône et
## infobulle d'explication (F2).
func _make_gauge(value: float, invert: bool, label_text: String, key: String = "") -> Control:
	var holder := RichPanel.new()
	holder.add_theme_stylebox_override("panel", StyleBoxEmpty.new())
	holder.mouse_filter = Control.MOUSE_FILTER_STOP
	holder.tooltip_text = RichTooltip.gauge(key, value) if key != "" else label_text
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 0)
	box.custom_minimum_size = Vector2(46, 0)
	box.mouse_filter = Control.MOUSE_FILTER_IGNORE
	holder.add_child(box)
	var caption := HBoxContainer.new()
	caption.alignment = BoxContainer.ALIGNMENT_CENTER
	caption.add_theme_constant_override("separation", 2)
	caption.mouse_filter = Control.MOUSE_FILTER_IGNORE
	if key != "":
		caption.add_child(IconLibrary.make_rect("gauge_" + key, 12.0))
	var caption_label := Label.new()
	caption_label.text = label_text
	UiType.apply(caption_label, UiType.CAPTION)
	caption.add_child(caption_label)
	box.add_child(caption)
	var track := ColorRect.new()
	track.color = Color(0.55, 0.50, 0.40)
	track.custom_minimum_size = Vector2(44, 10)
	track.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var fill := ColorRect.new()
	fill.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var ratio := clampf(value / 100.0, 0.0, 1.0)
	fill.color = _gauge_color(ratio, invert)
	fill.size = Vector2(44.0 * ratio, 10.0)
	fill.position = Vector2.ZERO
	track.add_child(fill)
	box.add_child(track)
	var value_label := Label.new()
	value_label.text = "%d" % int(round(value))
	UiType.apply(value_label, UiType.CAPTION)
	value_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	box.add_child(value_label)
	return holder


static func _gauge_color(ratio01: float, invert: bool) -> Color:
	var good := Color(0.28, 0.55, 0.22)
	var bad := Color(0.70, 0.16, 0.12)
	return good.lerp(bad, ratio01) if invert else bad.lerp(good, ratio01)


