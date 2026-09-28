class_name UiType
extends RefCounted

## Chantier PO2 (ADR 0097, bible DA § 12.2) : échelle typographique unique de la tranche —
## quatre variations de type du thème `parchment_theme.tres`. Tailles de base à la hauteur de
## référence de 900 px ; `Settings` les multiplie ensuite par l'échelle d'interface (×1,2 à
## 1080p) et par « Taille du texte ». Remplace les `add_theme_font_size_override` au hasard des
## écrans migrés : `UiType.apply(control, UiType.HEADING)` pose la bonne taille (et, pour un
## `Label`/`RichTextLabel`, la variation de type du thème pour la police et la couleur).

## Titre d'écran ou de fenêtre modale (26 px de base).
const TITLE := "Title"
## Titre de panneau, de section, nom de cité (20 px de base).
const HEADING := "Heading"
## Texte courant (17 px de base, taille par défaut du thème).
const BODY := "Body"
## Légendes, chiffres secondaires, nom de bourg (14 px de base — rien en dessous).
const CAPTION := "Caption"

const _BASE_SIZES := {
	TITLE: 26,
	HEADING: 20,
	BODY: 17,
	CAPTION: 14,
}


## Taille de base (900 px de référence) de `variation`. `BODY` par défaut si inconnue.
static func size(variation: String) -> int:
	return _BASE_SIZES.get(variation, _BASE_SIZES[BODY])


## Pose `variation` sur `control` : taille de police (clé selon la classe) et, pour un `Label`
## ou un `RichTextLabel`, la variation de type du thème (police et couleur de
## `parchment_theme.tres`). Ne touche pas aux couleurs ni polices posées explicitement par
## l'appelant : celles-ci restent prioritaires sur le thème.
static func apply(control: Control, variation: String) -> void:
	if control == null:
		return
	var key := "normal_font_size" if control is RichTextLabel else "font_size"
	control.add_theme_font_size_override(key, size(variation))
	if control is Label or control is RichTextLabel:
		control.theme_type_variation = &"" if variation == "" else StringName(variation)
