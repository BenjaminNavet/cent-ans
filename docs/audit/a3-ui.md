# Audit A3 : interface et expérience joueur (session 7, nuit du 24/09)

Auditeur : A3 (lecture seule). État audité : `main` à `70741c1`, avec la bibliothèque `game/bin/libcent_ans.debug.dylib`
présente sur le disque (compilée le 24/09 à 22:45). Références : Total War Three Kingdoms (3K), Pharaoh (PH), Warhammer III (WH3).
Audit précédent : `docs/design/2026-09-23-audit-ui-total-war.md` (19 défauts, § 7 ci-dessous).

## 1. Méthode et limites

- Parcours d'un nouveau joueur (France, graine 1337) avec les mises en scène existantes : `start_menu.tscn -- --menu-stage=…`,
  `campaign_map.tscn -- --stage=…` (22 écrans), `--flow-stage=…` (pause, sauvegarde, réglages, confirmation, rapport, alertes),
  `battle.tscn` (`--deploy-shot`, mêlée, `--result-shot`, `--siege`), et les copies jetables de `tests/codex_screenshot.gd`,
  `c7_screenshot.gd` et `coinage_screenshot.gd`, redirigées vers le dossier de l'audit.
- Résolutions : 1440×900 (défaut), 1280×720, 1920×1080, 2560×1440, via un script jetable qui redimensionne la fenêtre
  avant de charger la scène.
- 50 captures sont rangées dans `docs/audit/captures/a3/`, préfixées par étape : `0x` menu, `1x` campagne, `2x` bataille, `3x` résolutions.
- Le son et les animations d'interface sont jugés d'après le code (`audio_director.gd`), sans écoute.

**Problèmes d'environnement rencontrés (à régler avant toute recapture)**
1. **Le cache Godot était périmé.** Au premier lancement, on obtenait `Parse Error: Identifier "RetinueRow" not declared` et
   le Codex ne compilait plus (`codex_bubbles.gd`). `godot --headless --path game --import` corrige le problème. L'import
   crée aussi deux fichiers `.uid` non versionnés (`game/shaders/road_line.gdshader.uid`, `game/tests/c7_screenshot.gd.uid`),
   comme les trois `.uid` non suivis déjà présents. Il faut les commiter.
2. **La dylib de `game/bin` ne correspond pas à `main`.** `CampaignSim` n'expose pas `get_agents`, et l'ordre `recruit_agent`
   est refusé (`unknown variant`). **Les agents (lot C6) sont donc invisibles** dans le jeu actuel : captures `11-agents*.png`
   vides, aucun jeton, aucun registre. Il faut relancer `core/build.sh`. Je ne l'ai pas fait (mandat de lecture seule, dylib partagée).
3. `--resolution WxH` n'a aucun effet : l'autoload `Settings` réapplique la résolution enregistrée (`video/resolution`) au démarrage.

## 2. Synthèse

L'identité visuelle est la vraie réussite : parchemin, sceaux de cire, miniatures du Codex et de la chronique, portraits, écran
de résultat de bataille. Elle soutient la comparaison avec le style « tapisserie » de 3K. La grammaire Total War est en place :
bandeau d'ost, sceau du chef, cloche de fin de saison, lettres scellées, minicarte, cartes d'unités de bataille par
« batailles ».

**Ce qui manque surtout, c'est la discipline d'ensemble.**
- Les fenêtres s'empilent sans règle, et la minicarte passe **par-dessus** les grands panneaux.
- Les textes sont petits et ne s'adaptent pas : à 2560×1440, l'interface occupe un tiers de l'écran.
- L'économie ne s'explique pas : trois chiffres de revenu différents, et aucun historique.
- Les fins de tour sont bruyantes (nouvelles étrangères) et ne disent pas ce qui compte pour le joueur.
- Plusieurs textes techniques s'affichent devant le joueur : `data/rules/vision.json`, tableaux Markdown des crédits, erreur serde brute.

Notes sur 5, face à la moyenne 3K / PH / WH3 (5 = au niveau) :

| Axe | Note | Commentaire bref |
|---|---|---|
| Cohérence du thème (manuscrit) | 4 | Très cohérent ; exceptions : onglets gris foncé du Codex, boutons « Politique / Relief » de la minicarte en gris système, crédits bruts |
| Hiérarchie visuelle | 2 | Panneaux centraux qui couvrent l'objet sélectionné ; tout au même poids typographique ; alertes sans libellé |
| Typographie et lisibilité | 2 | Corps 9 à 11 px fréquents (compétences, jauges de population, légendes du Codex) ; aucune échelle automatique |
| Infobulles | 3 | Riches (unités, bâtiments, Codex épinglable) ; absentes ou pauvres sur les pastilles d'alerte, les cartes de bataille et les boutons grisés |
| Raccourcis | 2 | Nombreux mais cachés (G agents, K Codex, O objectifs, L tutoriel) ; conflit AZERTY en bataille ; aucune réaffectation |
| Retours visuels et sonores | 2 | Un seul clic générique ; pas de son d'erreur, d'ordre ou d'alerte ; toasts peu visibles |
| Clics par action courante | 3 | Déplacement 2 clics (bien) ; construction et recrutement demandent un défilement dans un panneau étroit |
| Lisibilité de l'économie | 2 | « Solde », « Revenu (dernier tour) », « Revenu prévisionnel » : trois nombres sans lien affiché ; pas d'écart d'un tour à l'autre |
| Notifications et fin de tour | 2 | Rapport de saison sans économie ; lettres et bandeau envahis par les nouvelles étrangères |
| Adaptation à la résolution | 2 | Pas de `stretch` ; 1280×720 serré mais jouable ; 2560×1440 minuscule |
| Accessibilité | 1 | Ni taille de texte, ni mode daltonien (rouge/vert partout), ni réaffectation des touches, ni sous-titres |
| Français | 4 | Très propre ; restent des pluriels « (s) », une coquille, du jargon de développeur |

## 3. Parcours joueur : défauts constatés

Gravité : **B** bogue (faux, bloquant ou trompeur), **L** lisibilité ou ergonomie, **F** finition. La colonne « Capture »
renvoie à `docs/audit/captures/a3/`.

### 3.1 Menu, chargement, réglages, crédits

| # | G | Défaut | Capture | Fichier probable |
|---|---|---|---|---|
| M1 | L | Les cartes de faction sont translucides : on lit « Gand », « Paris », « Dijon » et « Nantes » **à travers** le texte des cartes (le défaut n° 5 de l'audit précédent réapparaît ici) | 01 | `start_menu.gd` / `start_menu.tscn` |
| M2 | F | Le bandeau « La guerre de Cent Ans — printemps 1337… » couvre l'étiquette « Londres (Westminster) » de l'illustration | 01 | `start_menu.gd::_layout` |
| M3 | F | Le bouton de la faction choisie, « Choisie », a l'air désactivé. Le champ « Graine » est du jargon (à renommer « Graine aléatoire » ou à ranger dans « Options avancées »). La ligne d'état « Simulation réelle — données chargées — 1 sauvegarde(s) » est un message de débogage | 01 | `start_menu.tscn` |
| M4 | B | Les crédits affichent le Markdown brut (`\| Auteur \| Icônes \|`, `\|---\|---\|`), une commande `uv run --project tools …` et un titre en double coupé (« Crédits » puis « Crédits — Cent Ans ») | 03 | `credits_screen.gd` (convertir les tableaux en `RichTextLabel` ou en grille) |
| M5 | L | Réglages : pas d'onglet « Accessibilité » ni « Commandes ». Une infobulle expose `data/rules/vision.json`. La case « Plein écran » décochée ressemble à un carré plein | 02 | `settings_menu.gd:163` |
| M6 | F | Écran de chargement propre, avec un conseil. Il gagnerait une miniature ou une citation de chroniqueur (PH et 3K en mettent) | 04 | `loading_screen.gd` |

### 3.2 Carte de campagne et HUD

| # | G | Défaut | Capture | Fichier probable |
|---|---|---|---|---|
| C1 | B | **La minicarte est dessinée au-dessus des grands panneaux** (technologies, encyclopédie). Elle masque le bouton de fermeture × et le rang 5 de l'arbre ; à 1280×720, le panneau des technologies **ne se ferme plus à la souris** | 11-tech, 31-tech-1280x720, 11-encyclopedia | `map_ui.gd::layout_hud` (ordre des enfants du `CanvasLayer`, ou masquer la minicarte comme pour la faction) |
| C2 | B | « Printemps 1337 — **tour 0** » : le premier tour devrait être le tour 1. Après une chronique, la barre affiche « Automne 1340 » sans numéro de tour | 10, 11-chronicle | `map_ui.gd` (date) |
| C3 | L | Les panneaux s'empilent encore : Cour + Province, Objectifs + Province, Diplomatie par-dessus tout. Aucune fermeture exclusive (le défaut n° 13 n'est pas corrigé) | 11-court, 11-objectives | `map_ui.gd`, `campaign_map.gd` |
| C4 | L | Le panneau de province est centré et couvre la province et l'armée sélectionnées. 3K, PH et WH3 ancrent ce panneau **en bas**, pour garder la carte visible | 11-province, 11-city | `province_panel.gd`, `map_ui.gd` |
| C5 | L | La largeur du panneau change entre Garnison et Ville (bord gauche à 578 px puis à 475 px). Les motifs de refus en rouge sont coupés (« culture locale inadaptée »). La liste de recrutement ne montre que deux lignes (défilement dans un défilement) | 11-province, 11-settlement | `province_panel.gd`, `settlement_panel.gd` |
| C6 | L | Les mini-jauges de population (Mécont., Santé, Richesse, Biens) sont en corps 8 à 9 px, sans valeur lisible (défaut n° 11 non corrigé) | 11-city | `province_panel.gd` |
| C7 | F | Présentation incohérente : « Gouverneur : — » avec deux-points, contrairement au tableau clé/valeur. « Contrôleur — » est du jargon (écrire « Occupée par : — »). Dans la fiche de colonie, la valeur « Province » est décalée d'un cran | 11-province, 11-settlement | `province_panel.gd`, `settlement_panel.gd` |
| C8 | L | Les étiquettes d'armée (« 620 », « 240 ») couvrent les noms de ville : « Pa(ris) », « Ga(nd) », « Mo(ns) », « Louv(ain) ». À 2560×1440, les noms se chevauchent (Cardiff et Bristol, Winchester et Chichester, Mons et Valenciennes) | 10, 30-2560 | `army_markers.gd`, `city_markers.gd`, `zoom_tiers.gd` |
| C9 | L | Les boutons de la barre du haut n'ont qu'une icône : impossible de savoir, sans survol, lequel ouvre la Cour, les Technologies ou les Objectifs. Les raccourcis (C, T, P, O, K, G) ne sont écrits nulle part sur les boutons. Le registre des agents (G) n'a **aucun bouton** | 10 | `map_ui.gd` (barre du haut) |
| C10 | L | Les pastilles d'alerte de la cloche n'ont ni libellé ni compteur visible (sauf un petit « 8 »). Une petite pastille « ⊙ » mystérieuse flotte en permanence au-dessus de la cloche. WH3 donne une infobulle et un texte à chaque icône | 12-flow-alerts, 11-chronicle | `end_turn_cluster.gd` |
| C11 | L | Carte diplomatique : on ne voit pas les couleurs (l'Angleterre en guerre n'est pas rouge), et la légende « rouge guerre, bleu allié… » est coupée par le panneau de province (défaut n° 9 non corrigé) | 11-diplomacy_map | `diplomacy_controller.gd`, shader de terrain |
| C12 | B | Mise en scène du siège : l'armée est bien « 620 siège », mais le bloc « Siège : vivres… / Donner l'assaut » (`army_actions`) ne s'affiche pas. L'aide (F1) renvoie encore au « panneau d'armée », qui a été supprimé. À vérifier en jeu : le joueur ne trouverait pas l'assaut | 11-siege, 11-help | `siege_controller.gd`, `map_ui.gd::layout_hud`, `help_controller.gd` |
| C13 | L | La fenêtre de confirmation de fin de tour met le focus sur « Terminer le tour », et ce style de focus ressemble à un bouton **désactivé** | 12-flow-confirm | `parchment_theme.tres` (style `focus`) |
| C14 | F | Au démarrage, l'Angleterre est très sombre, couverte de points roses (colonies ennemies ?) : on lit mal les villes anglaises. Aucune légende n'explique ce voile | 10, 11-map | brouillard / `settlement_layer.gd` |

### 3.3 Économie : « pourquoi mon revenu baisse ? »

| # | G | Défaut | Capture |
|---|---|---|---|
| E1 | B | Trois chiffres sans lien affiché : la barre du haut dit « Solde +3 063 », le panneau de faction « Revenu (dernier tour) +0 » et « Revenu prévisionnel +27 713 ». Le panneau liste les charges **sans signe ni total**, alors que 27 713 − 9 930 − 5 020 − 9 700 = 3 063. 3K affiche un tableau recettes / dépenses / solde, avec l'écart par rapport au tour précédent | 11-faction |
| E2 | L | Aucun historique : ni courbe du trésor, ni « −1 243 par rapport à la saison passée, dont −900 d'entretien des nouvelles troupes ». Aucune ventilation par province (PH en propose une par ressource) | 11-faction |
| E3 | F | Trois symboles monétaires : `℔` (U+2114, symbole de la livre de poids) dans six scripts, `₶` (livre tournois, conforme au design) dans le bandeau d'ost, « livres » dans les événements. Formats différents aussi : « (1000) » contre « (1 000) », « -6000 livres » avec un tiret à la place du signe moins | 10, 11-diplomacy, 11-chronicle |
| E4 | F | Le calcul du solde net est fait en GDScript (`map_ui.gd::set_treasury`). Il vaut mieux que `core/` expose `net_income` : les deux calculs finiront par diverger | code |
| E5 | F | « Matières Premières » a une majuscule fautive. « Monnaie saine » est coupé par la barre de défilement | 11-faction |

### 3.4 Personnages, Cour, arbre familial, compétences

| # | G | Défaut | Capture |
|---|---|---|---|
| P1 | L | Liste de la Cour : de gros boutons « Voir » répétés, alors que la ligne entière devrait être cliquable. Les statuts sont tronqués (« général de l'armée en Île-de-Fra… »). La fiche recouvre le journal (le journal passe devant la fiche, en bas à gauche) | 11-court, 11-skills |
| P2 | L | Arbre de compétences : tous les nœuds d'une branche portent la même icône (épées croisées), avec des libellés en corps 9 px (« Verrouillé · 2 pt »). Le bas de l'arbre est coupé | 11-skills |
| P3 | L | Les actions grisées (« Nommer gouverneur », « Marier ») ne disent pas **pourquoi** elles sont grisées, ni dans le libellé ni en infobulle. 3K l'écrit toujours | 11-skills, 13-c7 |
| P4 | F | « Conjoint(e) » : le sexe est connu, il faut écrire « Épouse » ou « Époux ». « Prestige 0 » pour Philippe VI en 1337 surprend (signalé à A2) | 11-skills |
| P5 | ok | Arbre familial : réussi (portraits, dates, zoom, recentrage). Il manque une mini-vue d'ensemble et la mise en évidence de l'héritier | 13-c7 |

### 3.5 Diplomatie, technologies, objectifs, aide, Codex, encyclopédie

| # | G | Défaut | Capture |
|---|---|---|---|
| D1 | L | Le champ « Tribut (positif = exigé, négatif = offert) » est une interface de programmeur : prévoir deux champs, ou un curseur avec « Exiger » / « Offrir ». Les barres de relation n'ont pas de nombre. « Puissance militaire : 9370 » n'a pas de séparateur de milliers | 11-diplomacy |
| D2 | F | Objectifs : « 3 province(s) encore occupée(s) » ; « Soumettre la Bourgogne — soumis » (écrire « soumise ») ; le « score actuel : 500 » n'est pas expliqué | 11-objectives |
| D3 | L | L'aide F1 est un mur de texte sans structure ni illustration. Elle contient une coquille (« Son.... ») et renvoie à des éléments disparus (le panneau d'armée) | 11-help |
| D4 | L | Deux encyclopédies coexistent : l'**Encyclopédie** (mécanique) et le **Codex** (histoire), avec des navigations différentes. Pour le joueur, c'est un doublon. WH3 et 3K n'en ont qu'une, avec des onglets. Il faut une seule fenêtre, avec des onglets « Règles » et « Histoire » | 11-encyclopedia, 13-codex |
| D5 | L | Codex : le texte d'aide « Rechercher un nom, un lieu, un mot… » est illisible (clair sur clair). Les onglets sont gris foncé, hors du thème. Les entrées non découvertes, en gris, se confondent avec les découvertes | 13-codex-window |
| D6 | F | Technologies : aucun filtre ni aucune recherche ; les nœuds verrouillés sont gris sur gris (contraste faible) | 11-tech |

### 3.6 Fin de tour, rapport, lettres, chronique, sauvegarde

| # | G | Défaut | Capture |
|---|---|---|---|
| T1 | B | Le rapport de saison place la perte de **Wissant** (une ville du joueur !) au même niveau qu'une naissance. Il n'a pas de rubrique « Trésor », « Armées » ni « Bâtiments achevés », et n'offre pas d'action directe (le petit « ⌖ » passe inaperçu) | 12-flow-report |
| T2 | L | Les lettres scellées et le bandeau du haut sont envahis de nouvelles **étrangères** (« Lucques tombe aux mains de Florence », « Alliance Vérone–Autriche », « Assaut de Venise contre Trévise »), avec 16 lettres en attente dès le tour 6. Il faut filtrer par distance ou par intérêt (voisins, alliés, ennemis), comme le fait WH3 | 12-flow-alerts |
| T3 | ok | Chronique : excellente (miniature, choix chiffrés). Il faudrait remplacer le « −6000 livres » brut par une icône et un montant formaté | 11-chronicle |
| T4 | F | Sauvegarde : le nom par défaut est `france_printemps_1337` (tirets bas) ; les dates sont au format ISO (« 2026-09-24 à 23:19:05 ») au lieu du français « 24 sept. 2026, 23 h 19 » | 12-flow-save |
| T5 | L | La fin de tour n'a aucune transition (3K et PH montrent un bandeau « Tour des autres factions » et suivent les armées ennemies visibles) | code |

### 3.7 Bataille

| # | G | Défaut | Capture |
|---|---|---|---|
| B1 | B | **Conflit de touches en AZERTY.** La caméra lit les touches physiques (W A S D, donc Z Q S D en AZERTY), mais les ordres du chef lisent `keycode` Z/X/V/B/N. En AZERTY, avancer la caméra (touche Z) **déclenche le « Cri de guerre »** | `battle_camera.gd:68`, `leader_orders_bar.gd:15,209` |
| B2 | L | Déploiement : le message d'erreur « Les Chevaliers doivent être placés dans votre zone de déploiement » s'affiche dès l'ouverture, et la zone de déploiement ne se voit pas sur le terrain | 21 |
| B3 | L | Le bandeau du bas fait 180 px de haut et reste vide aux trois quarts. Les cartes ne portent qu'un chiffre, sans nom. « Retraite générale » est à côté de « Halte », sans confirmation. Il n'y a pas de portrait du chef (le sceau du chef existe côté campagne) | 21, 22, 34 |
| B4 | L | Journal de bataille : les lignes sont doublées sans distinguer les régiments (« Les Archers à l'arc long d'Angleterre plantent leurs pieux » deux fois). WH3 regroupe (« 2 régiments… ») | 22, 24 |
| B5 | F | Pendant la recharge, le compteur « 78 s » s'écrit par-dessus le libellé « Cri de guerre » : les deux sont illisibles | 24 |
| B6 | F | Écran de résultat : un grand vide au centre ; les régiments homonymes ne se distinguent pas ; rien sur les captifs et les rançons, l'expérience du chef ou le butin (alors que les systèmes existent) | 23 |
| B7 | L | À 2560×1440, toute l'interface de bataille est minuscule (cartes de 60 px) | 34 |

### 3.8 Tutoriel

| # | G | Défaut | Capture |
|---|---|---|---|
| U1 | L | La bulle de l'étape 3 couvre le bandeau d'ost qu'elle commente. Le tutoriel compte 14 étapes, sans sommaire ni option « rappelez-le-moi plus tard » | 11-tutorial |
| U2 | F | La flèche rouge verticale traverse la carte. Une pastille en surbrillance autour de l'élément visé serait plus lisible, comme dans 3K | 11-tutorial |

## 4. Nombre de clics par action courante

| Action | Cent Ans | 3K / WH3 | Commentaire |
|---|---|---|---|
| Déplacer une armée | 2 (clic, clic droit) | 2 | Bien. Le chemin et le coût s'affichent au survol |
| Recruter dans la capitale | 2 + défilement (province → ligne) | 3 | Le défilement coûte plus cher que le clic : la liste est cachée sous la garnison |
| Construire | 3 + défilement (province → Ville → défiler → bâtiment) | 3 | Les emplacements de bâtiments devraient être visibles sans défiler |
| Savoir pourquoi le revenu baisse | survol du solde, puis panneau de faction ; **aucun écart par rapport au tour précédent** | 1 survol | Voir E1 et E2 |
| Donner un général à une armée | C → ligne → « Voir » → « Donner le commandement » → choisir : 4 à 5 | 2 (depuis l'armée) | Le sceau « Sans chef » pourrait ouvrir directement le choix |
| Aller au lieu d'une alerte | 1 clic (pastille) | 1 | Bien, mais les pastilles sont muettes (C10) |
| Proposer la paix | P → faction → saisir un nombre → proposer : 4 | 3 | Voir D1 |
| Sauvegarder vite | Échap → Sauvegarder… → Sauvegarder : 3 | 1 (F5 / Ctrl+S) | Aucune sauvegarde rapide |

## 5. Responsivité

- **1280×720** (`30-campagne-1280x720`, `31-*-1280x720`, `32`, `33`) : c'est jouable. La barre du haut est saturée (la date colle
  aux boutons, la jauge de recherche rétrécit). Les cartes du bandeau d'ost perdent leur nom. Le panneau de province ne montre plus
  que deux lignes de recrutement. Le panneau des technologies ne se ferme plus à la souris (C1).
- **1920×1080** : correct, mais l'interface est proportionnellement plus petite, et le vide du bandeau de bataille augmente.
- **2560×1440** (`30-campagne-2560x1440`, `31-faction-2560x1440`, `34`) : l'interface est **minuscule** (corps de 10 à 12 px sur
  un écran de 27 pouces). Le projet n'a pas de `display/window/stretch`. Il existe bien une « Échelle de l'interface » (100 % par
  défaut), mais elle n'est pas calculée automatiquement. WH3 et 3K ajustent l'échelle par défaut à la hauteur de l'écran.

## 6. Accessibilité

Il manque, par ordre d'impact :
- une échelle d'interface automatique (voir § 5), avec un réglage « Taille du texte » séparé ;
- un mode daltonien (relations et jauges en rouge / vert, barres de moral vertes ou orange, guerre en rouge sur la carte
  diplomatique) : prévoir des motifs ou des icônes en plus des couleurs ;
- la réaffectation des touches (au moins la disposition AZERTY / QWERTY, voir B1) ;
- l'arrêt des effets de caméra et de fondu ;
- des infobulles au clavier et un focus visible distinct de l'état désactivé (C13).

## 7. Suivi de l'audit du 23/09

| n° | État au 24/09 |
|---|---|
| 1, 3, 4, 10, 14, 15 | corrigés (aucune ligne de débogage, élisions correctes) |
| 2 | corrigé dans la barre du haut (solde net) ; **nouvelle incohérence** dans le panneau de faction (E1) |
| 5 | corrigé pour les panneaux de la carte ; **réapparaît** sur les cartes du menu (M1) |
| 6, 7 | améliorés (bannières, étiquettes) ; chevauchements encore présents (C8) |
| 9 | **non corrigé** (C11) |
| 11 | **non corrigé** (C5, C6) |
| 12 | corrigé (« Chronique » grisé seulement sans décision) ; le nom de la recherche est maintenant écrit en petit à côté de sa jauge |
| 13 | **non corrigé** (C3) |
| 16, 17 | largement corrigés (bannières, cartes compactes, minicarte, vitesses) ; restent B3 et B4 |

## 8. Lots proposés, classés par rapport impact / effort

Effort : S ≤ 2 h, M ≤ 1 journée d'agent, L plusieurs jours. Impact de 1 à 5.

| Rang | Lot | Contenu | Fichiers | Effort | Impact |
|---|---|---|---|---|---|
| 1 | **U0 Environnement** | `core/build.sh` pour une dylib alignée sur `main` (agents), commit des `.uid`, `--import` dans la procédure de reprise ; faire respecter `--resolution` quand il est passé en ligne de commande | `game/bin/`, `game/**/*.uid`, `settings.gd` | S | 5 |
| 2 | **U1 Empilement des fenêtres** | Z-order : la minicarte passe **sous** tout panneau modal ou large, ou se masque (C1). Fermeture exclusive des panneaux centraux (C3). Échap ferme le panneau du dessus. Bouton × toujours visible | `map_ui.gd` (`layout_hud`, `docked_panels`), `campaign_map.gd` | M | 5 |
| 3 | **U2 Bogues de saisie et de contrôle** | B1 (ordres du chef en `physical_keycode`, ou touches par défaut hors ZQSD), C12 (bloc de siège et assaut), C2 (tour 1), C13 (style `focus`), B5 (compteur de recharge) | `leader_orders_bar.gd`, `siege_controller.gd`, `map_ui.gd`, `parchment_theme.tres` | S | 5 |
| 4 | **U3 Économie lisible** | `core/` expose `net_income`, les rubriques signées et l'écart par rapport au tour précédent. Panneau de faction en tableau recettes / dépenses / solde ; infobulle du solde avec « par rapport à la saison passée » ; courbe du trésor sur 12 saisons ; symbole `₶` partout (E1 à E5) | `core/crates/sim-campaign` (économie), pont, `faction_panel.gd`, `map_ui.gd`, `hud_style.gd` | M | 5 |
| 5 | **U4 Échelle et responsivité** | `stretch` en mode `canvas_items` avec aspect `expand`, ou échelle automatique = hauteur / 900 bornée entre 0,9 et 1,6 ; « Taille du texte » séparée ; test de capture à 4 résolutions dans la chaîne | `project.godot`, `settings.gd`, `settings_menu.gd`, test `tests/ui_resolutions.gd` | M | 5 |
| 6 | **U5 Fin de tour utile** | Rapport de saison en rubriques « Vos terres » (pertes et prises en rouge, en premier), « Trésor », « Armées », « Constructions et recherches », puis « Le monde » ; filtre d'intérêt des lettres et du bandeau (voisins, alliés, ennemis, grandes puissances) ; pastilles d'alerte avec libellé et compteur ; bandeau « Tour des autres factions » | `season_report.gd`, `news_letters.gd`, `end_turn_cluster.gd`, `flow_controller.gd` | M | 4 |
| 7 | **U6 Panneau de province ancré en bas** | Panneau bas pleine largeur à la manière de 3K : onglets Garnison / Ville / Colonies, largeur fixe, recrutement en cartes horizontales (plus de défilement imbriqué), jauges de population lisibles avec valeurs, motifs de refus en infobulle | `province_panel.gd`, `settlement_panel.gd`, `map_ui.gd` | L | 4 |
| 8 | **U7 Barre du haut et raccourcis** | Libellé court ou lettre de raccourci sur chaque bouton, bouton « Agents (G) », bouton « Codex (K) » bien visible ; sauvegarde rapide F5 / F9 ; fiche des raccourcis générée depuis l'`InputMap` ; onglet « Commandes » dans les réglages avec AZERTY / QWERTY | `map_ui.gd`, `settings_menu.gd`, `help_controller.gd`, `project.godot` | M | 4 |
| 9 | **U8 Finitions textes et données** | M3, M4 (crédits mis en forme), M5 (plus de chemins `data/` dans l'interface), C7, D2 (pluriels exacts au lieu de « (s) »), P4, T4 (dates en français), coquille « Son.... », erreurs brutes du moteur remplacées par un message joueur ; textes de `HUD_TEXTS` sortis dans `data/ui/` | `start_menu.*`, `credits_screen.gd`, `settings_menu.gd`, `province_panel.gd`, `victory_controller.gd`, `save_load_dialog.gd`, `rich_tooltip.gd`, `agent_controller.gd` | S | 3 |
| 10 | **U9 HUD de bataille compact** | Bandeau bas à hauteur utile, sceau du chef à gauche, « Retraite générale » séparée et confirmée, noms des régiments en infobulle, zone de déploiement dessinée au sol, pas d'erreur à l'ouverture, journal regroupé ; écran de résultat enrichi (captifs, rançons, expérience, butin) | `battle_hud.gd`, `battle_scene.gd`, `leader_orders_bar.gd` | M | 4 |
| 11 | **U10 Personnages** | Ligne entière cliquable ; icône par compétence ; motif affiché sur les actions grisées ; « Sans chef » ouvre le choix du général ; héritier mis en évidence dans l'arbre | `court_panel.gd`, `character_sheet.gd`, `skill_tree_view.gd`, `family_tree_view.gd`, `general_seal.gd` | M | 3 |
| 12 | **U11 Codex et encyclopédie réunis** | Une seule fenêtre « Codex » avec les onglets Histoire / Règles (unités, bâtiments…), recherche commune, onglets au style parchemin, texte d'aide lisible | `codex_window.gd`, `encyclopedia.gd` | M | 3 |
| 13 | **U12 Accessibilité** | Mode daltonien (motifs et icônes sur les relations, le moral et la carte diplomatique), option « réduire les animations », contraste renforcé | `hud_style.gd`, `parchment_theme.tres`, shader de la carte diplomatique, `settings.gd` | M | 3 |
| 14 | **U13 Sons d'interface** | Sons distincts : ordre donné, ordre refusé, alerte, lettre reçue, recrutement, construction, sélection d'armée (piétinement), avec une banque CC0 (voir A4) ; plus de silence sur les refus | `audio_director.gd`, `game/assets/audio/sfx/` | S | 3 |
| 15 | **U14 Carte diplomatique et étiquettes** | Couleurs de relation lisibles (C11), légende complète, pas de chevauchement entre étiquettes d'armée et noms de ville (décalage de l'étiquette ou gestion des collisions), explication du voile sur les terres étrangères | `diplomacy_controller.gd`, `army_markers.gd`, `city_markers.gd`, `zoom_tiers.gd` | M | 3 |
| 16 | **U15 Tutoriel** | Bulles qui évitent l'élément visé, surbrillance au lieu de la flèche, sommaire des étapes, « Plus tard » | `tutorial.gd`, `tutorial_controller.gd`, `tutorial_steps.gd` | S | 2 |

Ordre conseillé : **U0 et U2** tout de suite (bogues, une heure ou deux) ; **U1, U3 et U4** dans la même vague, parce qu'ils
touchent tous `map_ui.gd` (un seul propriétaire pour éviter les conflits) ; puis U5, U7, U9 ; les autres ensuite.

## 9. Transmis aux autres audits

- **A2 (mécaniques)** : Philippe VI a 58 ans et règne encore en 1351 dans la capture de l'arbre (il meurt en 1350) ; « Prestige 0 »
  pour le roi de France en 1337 ; « Entraînement à l'arc long » dans l'arbre militaire **français** ; La Rochelle avec une
  fortification de niveau 6.
- **A5 (technique)** : dylib obsolète et cache `.godot` périmé (§ 1) ; `--resolution` ignoré ; 2 ressources et 4 instances encore
  en mémoire à la sortie (`ERROR: 2 resources still in use at exit`).
