class_name TutorialSteps
extends RefCounted

## F8 — textes du tutoriel des premiers tours (interface, pas des données de jeu) : 14 étapes,
## chacune avec un objectif vérifié par `TutorialController` (identifiant `id`), une cible
## (`target`, résolue par le contrôleur en contrôle d'interface ou point de la carte) et un
## conseil historique propre à la faction jouée (France, Angleterre, Bourgogne ; `GENERIC_ADVICE`
## sinon, avec la description de la faction en introduction). Les noms (dirigeant, capitale,
## suzerain, objectifs) viennent de `data/factions`, `data/characters` et de la simulation via
## le contexte passé à `steps` : `{army}`, `{capital}`, `{welcome}`, `{faction}`,
## `{objectives_title}`, `{objectives}`, `{liege}`.
##
## `manual` : étape validée par le bouton « Continuer » (introduction, conclusion).

const STEP_IDS := [
	"intro", "select_army", "move_army", "open_province", "city_tab", "build", "research",
	"diplomacy", "end_turn", "season_report", "chronicle", "tax", "governor", "outro",
]

## Texte commun de chaque étape : titre, consigne, objectif affiché, cible.
const BASE := {
	"intro": {
		"title": "{welcome}",
		"text": "Nous sommes au printemps 1337. Votre faction : {faction}. Ce guide vous accompagne pendant les premiers tours : chaque étape donne un objectif, une flèche montre où agir, l'étape suivante s'ouvre dès que l'objectif est rempli.\n\n[b]{objectives_title}[/b] :\n{objectives}",
		"objective": "Cliquez sur « Continuer ».",
		"target": "",
		"manual": true,
	},
	"select_army": {
		"title": "Votre armée",
		"text": "Les bannières sur la carte sont les armées. Cliquez (bouton gauche) sur la bannière de {army} : le bandeau d'ost s'ouvre en bas de l'écran avec ses unités ; à gauche, le sceau du chef montre son général, sa posture et ses points de mouvement.",
		"objective": "Sélectionner {army}.",
		"target": "royal_army",
	},
	"move_army": {
		"title": "Donner un ordre de marche",
		"text": "Armée sélectionnée, la bulle dorée au sol montre jusqu'où elle peut aller cette saison. Survolez le sol : le chemin s'affiche en vert pour ce tour, en rouge pour les tours suivants, avec son coût. Clic droit sur le sol : l'armée part aussitôt. Un clic droit sur une colonie y marche (siège d'une place ennemie), sur une armée ennemie l'attaque. Les grands fleuves ne se passent qu'aux ponts et aux gués.",
		"objective": "Déplacer une de vos armées (clic droit sur le sol).",
		"target": "royal_army",
	},
	"open_province": {
		"title": "Vos provinces",
		"text": "Cliquez sur la ville de {capital} : son panneau montre la garnison, le recrutement et les bâtiments de la ville ; le bouton « Province » (ou un clic dans la campagne alentour) ouvre le panneau de province : propriétaire, population, mécontentement. Si une armée stationne devant la ville, un second clic au même endroit passe de l'armée à la ville.",
		"objective": "Ouvrir le panneau d'une de vos villes ou provinces.",
		"target": "capital",
	},
	"city_tab": {
		"title": "La ville",
		"text": "Dans le panneau de la ville, l'onglet « Bâtiments » montre ses bâtiments et ce qu'on peut y construire ; dans le panneau de province, l'onglet « Ville » montre aussi les quatre classes (paysans, bourgeois, clergé, noblesse) avec leurs jauges et les ressources. Survolez une ligne pour l'infobulle détaillée.",
		"objective": "Ouvrir l'onglet « Bâtiments » de la ville (ou « Ville » de la province).",
		"target": "city_tab",
	},
	"build": {
		"title": "Construire",
		"text": "Sous « Construire » (panneau de la ville) ou « Constructible » (panneau de province), chaque ligne est un bâtiment avec son coût et sa durée : un clic sur la ligne lance le chantier. Le coût est prélevé tout de suite ; un échafaudage sur la carte marque le chantier jusqu'à son achèvement. Une seule construction à la fois par ville.",
		"objective": "Lancer une construction dans une de vos villes.",
		"target": "buildable",
	},
	"research": {
		"title": "Les technologies",
		"text": "La jauge de recherche de la barre du haut indique la technologie en cours. Ouvrez l'arbre (touche T ou clic sur la jauge) et choisissez une technologie disponible : sans recherche, les points de la saison sont perdus.",
		"objective": "Choisir une recherche.",
		"target": "research",
		"modal_ok": true,  # l'arbre des technologies est une fenêtre modale : le guide reste affiché
	},
	"diplomacy": {
		"title": "La diplomatie",
		"text": "Le panneau de diplomatie (bouton de la barre ou touche P) liste les puissances, leur attitude envers vous et ses raisons, les traités et la faveur pontificale. Avant chaque envoi, il indique si l'autre partie accepterait.",
		"objective": "Ouvrir la diplomatie.",
		"target": "diplomacy",
	},
	"end_turn": {
		"title": "Finir le tour",
		"text": "Un tour est une saison. Quand vos armées ont marché, cliquez sur la cloche « Fin de tour », en bas à droite (ou Entrée) : les autres puissances jouent l'une après l'autre, les sièges avancent, les villes produisent, et les marches trop longues pour une saison reprennent d'elles-mêmes au tour suivant.",
		"objective": "Terminer le tour.",
		"target": "end_turn",
	},
	"season_report": {
		"title": "Le rapport de saison",
		"text": "Le rapport de saison résume ce qui vous concerne : batailles et sièges, diplomatie, cour, royaume, chronique, puis les nouvelles du monde. Un clic sur une ligne place la caméra sur l'événement. Fermez le rapport une fois lu.",
		"objective": "Lire puis fermer le rapport de saison.",
		"target": "season_report",
	},
	"chronicle": {
		"title": "La chronique",
		"text": "Les grands événements (historiques ou aléatoires) demandent une décision. Le bouton « Chronique (n) » indique combien attendent ; vous avez deux saisons pour choisir, sinon le conseil choisit à votre place. S'il n'y a rien pour l'instant, revenez-y plus tard et passez l'étape.",
		"objective": "Ouvrir la chronique sur un événement.",
		"target": "chronicle",
	},
	"tax": {
		"title": "L'impôt",
		"text": "Cliquez sur l'écu de votre faction (en haut à gauche) : le panneau de faction détaille trésor, revenus, entretien et biens. Le taux d'imposition (Bas, Normal, Haut) change le revenu dès la saison suivante, mais un fardeau lourd attise le mécontentement.",
		"objective": "Changer le taux d'imposition.",
		"target": "tax",
	},
	"governor": {
		"title": "Nommer un gouverneur",
		"text": "Ouvrez la cour (bouton de la barre ou touche C), choisissez un personnage puis « Nommer gouverneur » et une province. Sa compétence de gouvernance agit sur l'impôt, la construction et l'ordre public. Le bouton « Cour » du panneau de province filtre les gouverneurs possibles.",
		"objective": "Nommer un gouverneur.",
		"target": "governor",
		"modal_ok": true,  # la cour est une fenêtre modale : le guide reste affiché
	},
	"outro": {
		"title": "À vous de jouer",
		"text": "Vous connaissez l'essentiel. L'encyclopédie (touche L ou Menu → Encyclopédie) décrit chaque unité, bâtiment, technologie, trait et mécanique ; l'aide (F1) rappelle les commandes ; les objectifs (O) suivent votre progression. Bonne campagne !",
		"objective": "Cliquez sur « Terminer ».",
		"target": "",
		"manual": true,
	},
}

## Conseils historiques par faction jouable (étape → texte).
const ADVICE := {
	"fac_france": {
		"intro": "Philippe VI, premier Valois, vient de confisquer la Guyenne (24 mai 1337) ; Édouard III le défie et revendique votre couronne. Le royaume est le plus peuplé d'Occident : votre force est le nombre.",
		"select_army": "L'ost royal repose sur la noblesse à cheval ; à Crécy (1346), sa charge se brisera sur les archers anglais.",
		"move_army": "Couvrez la Picardie et la Guyenne : c'est par la Flandre, la Somme et Bordeaux que viendront les Anglais.",
		"open_province": "Paris, la plus grande ville d'Occident, porte le trésor du royaume.",
		"city_tab": "Les bourgeois paient l'essentiel de l'impôt ; les paysans nourrissent les villes et fournissent les levées.",
		"build": "Un marché ou des moulins enrichissent la province ; les murailles de pierre usent les chevauchées anglaises.",
		"research": "Les pavois et les arbalètes à moufle répondent à l'arc long ; les compagnies d'ordonnance (1445) donneront au roi une armée permanente.",
		"diplomacy": "L'Écosse est votre alliée (Auld Alliance) ; la Flandre, votre vassale, dépend de la laine anglaise et pourrait basculer.",
		"end_turn": "La guerre est déclarée : surveillez la Manche, les Anglais peuvent débarquer à chaque saison.",
		"season_report": "Les pertes de provinces et les sièges apparaissent en premier : réagissez vite.",
		"chronicle": "L'Écluse (1340), Crécy (1346), la Peste noire (1348) : vos décisions peuvent infléchir le cours de l'histoire.",
		"tax": "Un impôt lourd remplit le trésor, mais la Jacquerie (1358) et les Maillotins (1382) rappellent le prix du fardeau fiscal.",
		"governor": "Confiez les provinces lointaines (Languedoc, Poitou) à des princes du sang expérimentés.",
		"outro": "Tenez Paris et Reims, reprenez la Guyenne : Castillon (1453) marque la fin de la guerre.",
	},
	"fac_england": {
		"intro": "Édouard III, petit-fils de Philippe le Bel par sa mère Isabelle, revendique la couronne de France. Le royaume est moins peuplé, mais ses archers et sa laine font sa force.",
		"select_army": "L'armée d'Édouard marie archers à l'arc long et hommes d'armes démontés, la tactique victorieuse d'Halidon Hill (1333).",
		"move_army": "Traverser la Manche est possible entre deux ports : marchez jusqu'au port, puis clic droit sur le port d'arrivée ; la traversée prend la saison, et débarquer en terre ennemie coûte des hommes : visez la Guyenne amie ou un port flamand.",
		"open_province": "Londres et Westminster : le Parlement y vote les subsides de la guerre.",
		"city_tab": "La laine anglaise fait vivre les drapiers flamands : c'est votre meilleur levier diplomatique.",
		"build": "Un port facilite les traversées ; des buttes de tir entretiennent l'adresse des archers.",
		"research": "L'arc long est votre arme ; la poudre et les bombardes changeront la donne au siècle suivant.",
		"diplomacy": "La Flandre (Jacques van Artevelde, 1338) et les princes d'Empire sont vos alliés naturels ; l'Écosse est l'alliée de la France.",
		"end_turn": "Les grandes chevauchées ravagent le pays ennemi : posture « Chevauchée » sur le sceau du chef, en bas à gauche.",
		"season_report": "Surveillez l'Écosse : ses raids frappent le nord pendant que vous guerroyez en France.",
		"chronicle": "Crécy (1346), Poitiers (1356), Azincourt (1415) : les grandes victoires attendent vos décisions.",
		"tax": "Le Parlement consent l'impôt ; la révolte des Paysans (1381) est née d'une capitation trop lourde.",
		"governor": "La Guyenne est lointaine : un sénéchal compétent y vaut une armée.",
		"outro": "Faites-vous sacrer à Reims, ce qu'Henri VI ne put faire (couronné à Paris en 1431), et gardez l'héritage des Plantagenêts.",
	},
	"fac_burgundy": {
		"intro": "Eudes IV, beau-frère de Philippe VI, tient un duché riche mais modeste. La Bourgogne grandira par les mariages et les héritages jusqu'à devenir, sous les Valois, la puissance du « grand-duc d'Occident ».",
		"select_army": "L'armée ducale est petite : ménagez-la, chaque unité coûte cher à remplacer.",
		"move_army": "Restez prudent au début : la Flandre viendra par mariage (1369) et héritage (1384) plutôt que par la guerre.",
		"open_province": "Dijon, capitale du duché, au cœur des vignobles et des foires de Chalon.",
		"city_tab": "Clergé et bourgeois sont riches en Bourgogne : les abbayes de Cîteaux et Cluny pèsent dans la province.",
		"build": "Pressoirs et foires rapportent vite ; les murailles protégeront des Grandes Compagnies.",
		"research": "Comptabilité et lettres de change : la cour de Bourgogne sera la plus riche d'Occident.",
		"diplomacy": "Vassal du roi de France, vous pouvez choisir votre camp : Philippe le Bon s'alliera aux Anglais après Montereau (1419) avant la paix d'Arras (1435).",
		"end_turn": "Laissez France et Angleterre s'épuiser : chaque saison de paix enrichit le duché.",
		"season_report": "Les successions des princes voisins sont des occasions : lisez la rubrique « Le monde ».",
		"chronicle": "La querelle des Armagnacs et des Bourguignons (1407) fera de vous l'arbitre du royaume.",
		"tax": "Les villes flamandes, riches et turbulentes, supportent mal l'impôt : Gand se révoltera de 1449 à 1453.",
		"governor": "Un gouverneur habile tient les Pays-Bas pendant que le duc négocie à Paris.",
		"outro": "Indépendance, Pays-Bas et lien lorrain : l'échéance est 1477, l'année de Nancy.",
	},
}

## Conseils des factions sans texte propre (étape → texte) : généralités du XIVᵉ siècle.
## L'introduction reprend la description de la faction (`data/factions`) ; `diplomacy_vassal`
## remplace `diplomacy` quand la faction a un suzerain.
const GENERIC_ADVICE := {
	"select_army": "Au XIVᵉ siècle, une armée se lève pour une campagne et coûte cher à tenir : ménagez la vôtre, elle ne se remplace pas en une saison.",
	"move_army": "Fleuves, ponts et places fortes commandent les routes : qui les tient choisit où l'on se bat.",
	"open_province": "{capital} est le cœur de votre domaine : la perdre, c'est souvent tout perdre.",
	"city_tab": "Paysans, bourgeois, clergé, noblesse : quatre ordres dont les intérêts s'accordent rarement.",
	"build": "Moulins, marchés et halles paient les guerres de demain ; les murailles donnent le temps d'être secouru.",
	"research": "Le siècle change la guerre : l'arc long et l'arbalète, puis la poudre, entament la suprématie du chevalier.",
	"diplomacy": "Sans suzerain au-dessus de vous, vos alliances et vos mariages sont votre meilleure garantie.",
	"diplomacy_vassal": "Votre suzerain : {liege}. L'hommage oblige, mais un vassal habile choisit ses querelles.",
	"end_turn": "Quatre saisons font une année : les grands princes comptent en décennies, pas en campagnes.",
	"season_report": "Lisez aussi les nouvelles du monde : les malheurs des voisins sont des occasions.",
	"chronicle": "La Peste noire (1347-1352) frappera presque toute l'Europe : peu de décisions pèseront autant que celles de ces années-là.",
	"tax": "Partout l'impôt nouveau fait gronder : villes et campagnes se soulèvent quand le fardeau paraît injuste.",
	"governor": "Un prince ne peut être partout : baillis, sénéchaux et capitaines tiennent le pays en son nom.",
	"outro": "Les objectifs de votre titre se lisent sur votre écu, en haut à gauche : section « Féodalité » du panneau de faction.",
}

## Nom de l'armée principale selon la faction (texte d'interface).
const ARMY_NAMES := {"fac_france": "l'ost royal", "fac_england": "l'armée royale", "fac_burgundy": "l'armée ducale"}


## Étapes pour `faction_id` ; `context` : `{faction, ruler, capital, objectives, end_year,
## objectives_title, liege, description}` (tout est facultatif).
static func steps(faction_id: String, context: Dictionary = {}) -> Array[Dictionary]:
	var ruler := str(context.get("ruler", ""))
	var liege := str(context.get("liege", ""))
	var values := {
		"army": str(ARMY_NAMES.get(faction_id, "votre armée principale")),
		"faction": str(context.get("faction", "votre royaume")),
		"welcome": "Bienvenue, %s" % ruler if ruler != "" else "Bienvenue",
		"capital": str(context.get("capital", "votre capitale")),
		"objectives": str(context.get("objectives", "• Survivre et prospérer.")),
		"objectives_title": str(context.get("objectives_title", "Vos objectifs historiques (avant %s)" % str(context.get("end_year", "1453")))),
		"liege": liege,
	}
	var advice: Dictionary = ADVICE.get(faction_id, {})
	if advice.is_empty():
		advice = GENERIC_ADVICE.duplicate()
		advice["intro"] = str(context.get("description", ""))
		if liege != "":
			advice["diplomacy"] = advice["diplomacy_vassal"]
	var result: Array[Dictionary] = []
	for step_id in STEP_IDS:
		var base: Dictionary = BASE[step_id]
		result.append({
			"id": step_id,
			"title": str(base["title"]).format(values),
			"text": str(base["text"]).format(values),
			"objective": str(base["objective"]).format(values),
			"advice": str(advice.get(step_id, "")).format(values),
			"target": str(base.get("target", "")),
			"manual": bool(base.get("manual", false)),
			"modal_ok": bool(base.get("modal_ok", false)),
		})
	return result


static func count() -> int:
	return STEP_IDS.size()
