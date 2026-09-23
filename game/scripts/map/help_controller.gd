class_name HelpController
extends Node

## Aide en jeu (F1 ou Menu → Aide) : commandes et principes du jeu, en français. Séparé de
## `campaign_map.gd` ; celui-ci n'appelle que `setup` et `handle_input`.

const HELP_TEXT := """[b]Commandes de la carte[/b]
• Déplacer la caméra : W A S D (Z Q S D en AZERTY), flèches ou bords d'écran (F2 pour désactiver) ; molette : zoom ; Q / E : rotation.
• Clic gauche : sélectionner une armée ou une province. Clic droit (armée sélectionnée) : ordre de déplacement.
• Entrée : fin du tour. Échap : désélectionner.
• C : cour et personnages. T : technologies. P : diplomatie. O : objectifs. F1 : cette aide.
• Modes de carte : M mécontentement, N diplomatie, R religion. F12 : capture d'écran.
• Volumes de la musique et des effets : menu de départ ou Menu → Son….

[b]La campagne[/b]
• Un tour est une saison. L'hiver réduit les déplacements et affame les armées en pays ennemi.
• Les provinces rapportent selon leur population, leurs bâtiments et l'impôt (panneau de faction, clic sur le blason). La cour et l'administration coûtent d'autant plus que le royaume est vaste.
• Recrutez dans le panneau de province, formez des armées, donnez-leur un général (fiche personnage).
• Chronique : les grands événements historiques (Crécy, la Peste noire, Jeanne d'Arc…) et des événements aléatoires demandent une décision ; bouton « Chronique (n) » de la barre, deux tours pour choisir.
• Posture « Siège » : l'armée assiège la place ennemie ; vivres, brèche et bouton « Donner l'assaut » apparaissent dans le panneau d'armée. Posture « Chevauchée » : pillage et butin.
• Quand vos armées rencontrent l'ennemi, choisissez « Livrer bataille » (bataille 3D) ou la résolution automatique.

[b]Batailles[/b]
• Espace : pause (ordres possibles en pause). 1 / 2 / 3 : vitesse ×1 / ×2 / ×4.
• Clic gauche : sélection (glisser : rectangle, Maj : ajouter). Clic droit : déplacer ou attaquer ; double clic droit : au pas de course ; glisser-droit : orienter la ligne.
• F : formation, G : tir à volonté, H : halte. La pluie gêne les archers, les flancs et les arrières sont vulnérables, le moral s'effondre sans général.

[b]Personnages, diplomatie, religion[/b]
• Les personnages gagnent de l'expérience et des points de compétence (arbre à trois branches), se marient, ont des enfants, meurent : la succession suit la loi du royaume.
• La diplomatie montre, avant envoi, si l'autre partie accepterait et pourquoi. Une guerre sans casus belli ou la rupture d'une trêve coûtent cher en réputation.
• La faveur du pape se gagne par la piété et les dons ; l'excommunication est un fléau. Le Grand Schisme (1378) oblige à choisir une obédience.

[b]Victoire[/b]
• Chaque grande faction a des objectifs historiques (touche O). Les remplir tous avant l'échéance donne la victoire ; perdre toutes ses terres, la défaite."""

var map: Node = null  # CampaignMap
var panel: PanelContainer


func setup(campaign_map: Node) -> void:
	map = campaign_map
	panel = PanelContainer.new()
	panel.theme = load("res://scenes/ui/parchment_theme.tres")
	panel.set_anchors_preset(Control.PRESET_CENTER)
	panel.custom_minimum_size = Vector2(900, 640)
	panel.offset_left = -450
	panel.offset_right = 450
	panel.offset_top = -320
	panel.offset_bottom = 320
	var box := VBoxContainer.new()
	panel.add_child(box)
	var header := HBoxContainer.new()
	var title := Label.new()
	title.text = "Aide — Cent Ans"
	title.add_theme_font_size_override("font_size", 22)
	title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	header.add_child(title)
	var close := Button.new()
	close.text = "×"
	close.pressed.connect(func() -> void: panel.hide())
	header.add_child(close)
	box.add_child(header)
	var text := RichTextLabel.new()
	text.bbcode_enabled = true
	text.text = HELP_TEXT
	text.size_flags_vertical = Control.SIZE_EXPAND_FILL
	text.add_theme_font_size_override("normal_font_size", 15)
	text.add_theme_font_size_override("bold_font_size", 17)
	box.add_child(text)
	map.ui.add_child(panel)
	panel.hide()
	var popup: PopupMenu = map.ui.menu_button.get_popup()
	popup.add_item("Aide (F1)", 901)
	popup.id_pressed.connect(func(id: int) -> void:
		if id == 901:
			toggle())


func toggle() -> void:
	panel.visible = not panel.visible


func handle_input(event: InputEvent) -> bool:
	if event is InputEventKey and event.pressed and not event.echo and (event as InputEventKey).physical_keycode == KEY_F1:
		toggle()
		return true
	return false
