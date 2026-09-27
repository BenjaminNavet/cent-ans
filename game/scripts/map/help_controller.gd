class_name HelpController
extends Node

## Aide en jeu (F1 ou Menu → Aide) : commandes et principes du jeu, en français. Séparé de
## `campaign_map.gd` ; celui-ci n'appelle que `setup` et `handle_input`.

## Principes du jeu ; la fiche des commandes de la carte, en tête, est générée depuis
## l'InputMap (`ShortcutSheet`, lot U7) à chaque ouverture (disposition du clavier à jour).
const HELP_TEXT := """[b]La campagne[/b]
• Un tour est une saison. L'hiver réduit les déplacements et affame les armées en pays ennemi ; un pays dévasté les nourrit mal, même chez soi.
• Les armées traversent la mer entre deux ports ; débarquer en terre ennemie épuise le mouvement et coûte {rule.landing_loss_percent} % des hommes ({rule.landing_loss_winter_percent} % l'hiver).
• Les provinces rapportent selon leur population, leurs bâtiments et l'impôt (panneau de faction, clic sur le blason). La cour et l'administration coûtent d'autant plus que le royaume est vaste et que le trésor dort ({rule.opulence_percent} % de l'excédent au-delà de {rule.opulence_seasons} saisons de revenu). En dette, toutes les troupes perdent {rule.bankruptcy_morale_penalty} de moral par saison, sans déserter : licenciez.
• Recrutez dans le panneau de province, formez des armées, donnez-leur un général (fiche personnage).
• Chronique : les grands événements historiques (Crécy, la Peste noire, Jeanne d'Arc…) et des événements aléatoires demandent une décision ; bouton « Chronique (n) » de la barre, {rule.decision_turns} tours pour choisir.
• Posture « Siège » : l'armée assiège la place ennemie ; quand elle est sélectionnée, l'état du siège (vivres, brèche) et le bouton « Donner l'assaut » apparaissent au-dessus du bandeau d'ost, en bas de l'écran. Posture « Chevauchée » : pillage et butin.
• Quand vos armées rencontrent l'ennemi, choisissez « Livrer bataille » (bataille 3D) ou la résolution automatique.

[b]Batailles[/b]
• Espace : pause (ordres possibles en pause). + / − ou boutons en bas à droite : vitesse ×1 / ×2 / ×4. Ctrl+1…9 : enregistrer un groupe, 1…9 : le rappeler (deux fois : centrer).
• Clic gauche : sélection (glisser : rectangle, Maj : ajouter). Clic droit : déplacer ou attaquer ; double clic droit : au pas de course ; glisser-droit : orienter la ligne.
• F : tir à volonté, T : formation, H : halte ; modes R (course), G (garde), K (escarmouche), M (mêlée des tireurs). Ordres du chef : Z X V B N (W X V B N en AZERTY). C : suivre la sélection. U : bannières. F1 : aide de bataille. La pluie gêne les archers, les flancs et les arrières sont vulnérables, le moral s'effondre sans général.

[b]Personnages, diplomatie, religion[/b]
• Les personnages gagnent de l'expérience et des points de compétence (arbre à trois branches), se marient, ont des enfants, meurent : la succession suit la loi du royaume.
• La diplomatie montre, avant envoi, si l'autre partie accepterait et pourquoi. Une guerre sans casus belli ou la rupture d'une trêve coûtent cher en réputation.
• La faveur du pape se gagne par la piété et les dons ; l'excommunication est un fléau. Le Grand Schisme (1378) oblige à choisir une obédience.

[b]Victoire[/b]
• Chaque grande faction a des objectifs historiques (touche O). Les remplir tous avant l'échéance donne la victoire ; perdre toutes ses terres, la défaite."""

var map: Node = null  # CampaignMap
var panel: PanelContainer
var text: RichTextLabel


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
	# UX2 : le guide pas à pas se relance (ou reprend là où « Plus tard » l'a laissé) d'ici.
	var guide := Button.new()
	guide.name = "TutorialButton"
	guide.text = "Tutoriel pas à pas"
	guide.tooltip_text = "Reprend le tutoriel à l'étape où vous l'aviez laissé, sinon le relance depuis le début."
	guide.pressed.connect(func() -> void:
		panel.hide()
		var tutorial: Node = map.get("tutorial")
		if tutorial != null:
			tutorial.call("reopen"))
	header.add_child(guide)
	var close := Button.new()
	close.text = "×"
	close.pressed.connect(func() -> void: panel.hide())
	header.add_child(close)
	box.add_child(header)
	text = RichTextLabel.new()
	text.bbcode_enabled = true
	text.text = full_text()
	text.size_flags_vertical = Control.SIZE_EXPAND_FILL
	text.add_theme_font_size_override("normal_font_size", 15)
	text.add_theme_font_size_override("bold_font_size", 17)
	box.add_child(text)
	# BP1 : mots du Codex cliquables (bulles imbriquées) dans l'aide.
	var bubbles := map.get_node_or_null("/root/CodexBubbles")
	if bubbles != null:
		bubbles.call("attach", text)
	map.ui.add_child(panel)
	panel.hide()
	var popup: PopupMenu = map.ui.menu_button.get_popup()
	popup.add_item("Aide (F1)", 901)
	popup.id_pressed.connect(func(id: int) -> void:
		if id == 901:
			toggle())


func toggle() -> void:
	if not panel.visible:
		text.text = full_text()
	panel.visible = not panel.visible


## Fiche des raccourcis (InputMap) puis principes du jeu.
static func full_text() -> String:
	return "[b]Commandes de la carte[/b] (disposition du clavier : Réglages → Commandes)\n\n%s\n\n%s" % [ShortcutSheet.bbcode(), CodexText.format(RuleValues.format(HELP_TEXT), true)]


func handle_input(event: InputEvent) -> bool:
	if event.is_action_pressed("help_open") and not event.is_echo():  # U7 : action de l'InputMap
		toggle()
		return true
	return false
