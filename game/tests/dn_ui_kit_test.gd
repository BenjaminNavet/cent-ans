extends TestCase

## Lot DN ui-kit : le thème `parchment_theme.tres` expose les variations de type du kit
## d'enluminure (bouton principal, jauge, séparateur, cadres) et les widgets standards
## (onglets, curseur, barre de progression) portent leurs ornements.
##
## Usage : godot --headless --path game --script res://tests/dn_ui_kit_test.gd

const THEME_PATH := "res://scenes/ui/parchment_theme.tres"
const VARIATIONS := {
	"PrimaryButton": "Button",
	"OrnateProgress": "ProgressBar",
	"FleuronSeparator": "HSeparator",
	"InsetFrame": "PanelContainer",
	"InitialFrame": "PanelContainer",
}


func _init() -> void:
	var theme := load(THEME_PATH) as Theme
	check(theme != null, "le thème se charge")
	if theme != null:
		_check_variations(theme)
		_check_widgets(theme)
	finish()


func _check_variations(theme: Theme) -> void:
	for variation: String in VARIATIONS:
		check(theme.get_type_variation_base(variation) == StringName(VARIATIONS[variation]),
			"%s dérive de %s" % [variation, VARIATIONS[variation]])
	for state in ["normal", "hover", "pressed", "disabled", "hover_pressed"]:
		check(theme.has_stylebox(state, "PrimaryButton"), "PrimaryButton/%s" % state)
	var normal := theme.get_stylebox("normal", "PrimaryButton")
	check(normal != theme.get_stylebox("normal", "Button"), "le bouton principal diffère du bouton normal")
	check(theme.has_stylebox("panel", "InsetFrame") and theme.has_stylebox("panel", "InitialFrame"), "cadres")
	check(theme.has_stylebox("separator", "FleuronSeparator"), "séparateur enluminé")
	check(theme.has_stylebox("fill", "OrnateProgress") and theme.has_stylebox("background", "OrnateProgress"), "jauge")


func _check_widgets(theme: Theme) -> void:
	check(theme.get_stylebox("tab_selected", "TabContainer") != theme.get_stylebox("tab_unselected", "TabContainer"), "onglets actif/inactif distincts")
	check(theme.has_icon("grabber", "HSlider"), "poignée de curseur")
	check(theme.has_stylebox("slider", "HSlider"), "rail de curseur")
	for name in ["button_primary_normal", "tab_ornate_selected", "progress_frame", "separator_h", "inset_ornate", "initial_frame"]:
		check(ResourceLoader.exists("res://assets/ui/illumination/%s.png" % name), "texture %s" % name)

