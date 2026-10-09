extends "res://tests/smoke_battle.gd"

## Smoke test headless :
##  1. GDExtension : CampaignSim joue 10 tours, vérifie la date ;
##  2. scène de carte sur les fixtures synthétiques (`CENT_ANS_DATA_DIR` → tests/fixtures) :
##     tuiles de terrain, rivières, marqueurs, picking de la province 3 ;
##  3. écran de démarrage : 3 cartes de faction ;
##  4. boucle de campagne (France, via SimFacade, réelle ou factice — imprimé) : sélection de la
##     première armée du joueur, provinces atteignables non vides, ordre de déplacement vers la
##     première atteignable, 4 fins de tour, sauvegarde `user://saves/smoke.json`, rechargement,
##     égalité des dates. Sur les vraies données `data/` si la sim réelle est disponible.
##  5. personnages et dynasties (M4, simulation sur
##     `data/` — imprimé) : panneau de cour (≥ 1 personnage), `debug_grant_xp` + `learn_skill`
##     sur le dirigeant, `assign_governor` d'un courtisan à `prov_normandie` (ou la première
##     province contrôlée ≠ capitale), `propose_marriage` entre deux candidats valides (ou skip
##     explicite si aucun), 40 fins de tour → au moins une naissance ou une mort au journal.
##  6. technologies (M6, vraie simulation si elle expose `get_tech_tree`, sinon skip imprimé) :
##     panneau des technologies (un nœud par technologie), ordre `research` sur la technologie
##     disponible la moins chère, `get_research` non vide, refus d'une technologie connue,
##     20 fins de tour → au moins une technologie acquise et un événement `technology_researched`.
##  8. batailles (M7) : BattleSim headless (≤ 12 000 ticks, fin, resolve_battle), puis dialogue
##     d'avant-bataille → battle.tscn (60 images) → écran de fin → retour à la carte.
##  9. chronique (M10, vraie simulation si elle expose `get_pending_decisions`, sinon skip) :
##     60 tours France, au moins un événement historique et un aléatoire proposés au joueur, une
##     décision résolue par l'ordre `choose_event_option` (les autres par la méthode dédiée),
##     fenêtre `ChronicleWindow` instanciée sur une décision réelle.
##  M10 assets : autoload AudioDirector (effets, musiques, persistance des volumes dans
##     `user://settings.cfg`), écus et portraits (`PortraitLoader`), modèles 3D (`ModelLibrary`,
##     marqueur d'armée habillé) et replis quand un fichier manque.
## 11. bataille de siège (M8 § 2) : assaut français de la Guyenne (`debug_stage_siege`),
##     murailles (`get_siege`), défenseurs sur le rempart, IA des deux camps jusqu'à la fin,
##     `resolve_battle` ; puis battle.tscn sur un siège (murailles maillées, 40 images).
## 12. flow (F3) : réglages écrits et relus (volumes conservés), écran de chargement jusqu'à la
##     carte, sauvegarde automatique tournante (auto_1..3, les fichiers du joueur sont mis de
##     côté puis restaurés), rapport de saison non vide après quelques tours, alertes, menu
##     pause ouvert puis fermé (arbre en pause), dialogue de sauvegarde, crédits.
## 13. codex (H2) : fiches de `data/codex`, liens `[[…]]` et auto-liens, pile de 3 bulles (dont
##     une ouverte par survol simulé), découvertes, fermeture après la grâce, fenêtre Codex.
## 14. tutorial/encyclopedia (F8) : tutoriel France sur la vraie simulation, chaque objectif
##     rempli par l'interface ou un ordre fait avancer l'étape (sélection, marche, province,
##     onglet Ville, construction, recherche, diplomatie, fin de tour, rapport, chronique,
##     impôt, gouverneur), progression persistée ; encyclopédie : chaque onglet > 0 entrée,
##     fiche non vide, recherche filtrée, liens internes, retour, touche L.
## 15. table/médecine (H9) : section Table (changement de régime par l'UI, refus affiché,
##     lecture seule), ligne de budget, infobulle de tech médecine, genres table/medicine.
## 16. monnaie/rançons (H11) : changement de monnaie par l'UI et refus du second dans l'année,
##     budget, ordre de chevalerie (refus affiché), panneau des rançons (données simulées), genres
##     coinage/ransom/chivalry, liens Encyclopédie ↔ Codex. Seule : CENT_ANS_SMOKE_ONLY=coinage_ransom.
## 17. agents (C6, vraie simulation si elle expose `get_agents`, sinon skip imprimé) : recrutement
##     d'un espion par le registre (G), jeton sur la carte, sélection → anneaux et barre d'actions
##     (4 actions avec pourcentage), déplacement par ordre, contre-espionnage dans une colonie amie
##     → rapport, ligne « agent » au journal de la saison suivante, onglet Agents de l'encyclopédie.
## Usage : godot --headless --path game --script res://tests/smoke.gd
## Code de sortie 0 si tout passe, 1 sinon.
##
## GT7 : le corps des sections vit dans `smoke_<thème>.gd`, chaînés par héritage
## (base → carte → campagne → interface → bataille) ; ce fichier n'enchaîne que les sections.


func _init() -> void:
	# Doit précéder l'instanciation de l'autoload MapPaths (après _init).
	_fixtures_dir = ProjectSettings.globalize_path("res://tests/fixtures")
	OS.set_environment(MapPaths.ENV_VAR, _fixtures_dir)
	_test_root = "user://smoke_%d_%d" % [OS.get_process_id(), Time.get_ticks_usec()]
	_test_settings_path = _test_root.path_join("settings.cfg")
	_test_codex_path = _test_root.path_join("codex.json")
	_test_saves_dir = _test_root.path_join("saves")
	_run_campaign_sim()
	await process_frame
	facade = root.get_node("/root/SimFacade")
	paths = root.get_node("/root/MapPaths")
	facade.call("use_test_saves_dir", _test_saves_dir)
	SaveSlots.use_test_dir(_test_saves_dir)
	# F3 : réglages par défaut sur un fichier dédié à cette exécution (le fichier du joueur
	# n'est pas touché), sans sauvegarde automatique hors de l'étape « flow ».
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file", _test_settings_path)
		settings.call("set_value", "game/autosave_interval", 0, false)
		settings.call("set_value", "tutorial/enabled", false, false)  # F8 : seulement à l'étape 13
	# Exécution ciblée d'une étape : CENT_ANS_SMOKE_ONLY=coinage_ransom.
	if OS.get_environment("CENT_ANS_SMOKE_ONLY") == "coinage_ransom":
		await _run_coinage_ransom()
		_cleanup_test_dir()
		finish()
		return
	if OS.get_environment("CENT_ANS_SMOKE_ONLY") == "codex_bubbles":
		await _run_codex_bubbles_b1()
		_cleanup_test_dir()
		finish()
		return
	# Batailles seules (bataille rangée avec déploiement, puis siège) : CENT_ANS_SMOKE_ONLY=battle.
	if OS.get_environment("CENT_ANS_SMOKE_ONLY") == "battle":
		await _run_battle()
		await _run_siege_battle()
		_cleanup_test_dir()
		finish()
		return
	if OS.get_environment("CENT_ANS_SMOKE_ONLY") == "ui_layout":
		await _run_ui_layout()
		finish()
		return
	# Tutoriel et encyclopédie seuls (F8) : CENT_ANS_SMOKE_ONLY=tutorial.
	if OS.get_environment("CENT_ANS_SMOKE_ONLY") == "tutorial":
		await _run_tutorial()
		_cleanup_test_dir()
		finish()
		return
	await _run_campaign_map()
	await _run_start_menu()
	await _run_campaign_loop()
	await _run_minimap_fog()  # C1
	await _run_ui_layout()  # UI2 : pile des panneaux (U1), échelle et 4 résolutions (U4)
	await _run_agents()  # C6 agents
	_run_city_economy()
	await _run_characters()
	await _run_technologies()
	await _run_diplomacy()
	await _run_trade()  # C5
	_run_sea_lanes()  # SL1
	await _run_battle()
	await _run_chronicle()
	await _run_assets()  # M10 assets
	await _run_icons()  # F2
	await _run_codex()  # H2
	await _run_codex_bubbles_b1()  # B1 bulles partout
	await _run_table_medicine()  # H9
	_run_edicts()  # C4 (TW)
	await _run_coinage_ransom()  # H11
	await _run_siege_battle()
	await _run_flow()  # F3
	await _run_tutorial()  # F8
	_cleanup_test_dir()
	finish()
