# Cent Ans — manuel du joueur

*Printemps 1337. Édouard III d'Angleterre revendique la couronne de France ; Philippe VI de Valois confisque
la Guyenne. Commence un siècle de guerre.*

Cent Ans est un jeu de grande stratégie solo : une **carte de campagne** au tour par tour, où chaque tour est
une saison, et des **batailles en temps réel avec pause** en 3D quand deux armées se rencontrent.

## 1. Installation et lancement

- **Application macOS** : `tools/export_macos.sh` produit `export/Cent Ans.app`, autonome (données
  embarquées). Double-cliquez ; au premier lancement, macOS peut demander d'autoriser l'application dans
  Réglages Système → Confidentialité et sécurité (signature ad hoc).
- **Depuis les sources** : `core/build.sh` (simulation Rust), `godot --headless --path game --import` une
  fois, puis `godot --path game`.
- Réglages, sauvegardes et progression du tutoriel sont rangés dans le dossier utilisateur de Godot
  (`~/Library/Application Support/Godot/app_userdata/Cent Ans/`).

## 2. Menu de départ

Choisissez votre couronne sur la carte ancienne :

| Faction | Situation en 1337 | Objectifs (échéance) |
|---|---|---|
| **Royaume de France** | le plus riche d'Occident, vassaux turbulents | bouter les Anglais, tenir Paris et Reims, reprendre la Guyenne, soumettre la Bourgogne (1453) |
| **Royaume d'Angleterre** | finances tendues, archers redoutés | sacre à Reims, héritage Plantagenêt, 15 provinces du royaume, soumettre l'Écosse (1453) |
| **Duché de Bourgogne** | prospère, vassal et beau-frère du roi de France | indépendance, Pays-Bas, lien lorrain (1477) |

- **Continuer** reprend la dernière sauvegarde ; **Charger une partie** ouvre les emplacements.
- La **graine** fixe le hasard : même graine et mêmes ordres donnent la même partie.
- **Réglages** et **Crédits** sont accessibles ici comme en jeu.

Vingt-cinq autres factions sont jouées par l'IA : Écosse, Flandre, Bretagne, Navarre, Castille, Aragon,
Portugal, Grenade, Papauté, Savoie, Milan, Gênes, Venise, Florence, Vérone, Naples, Empire, Bohême,
Autriche, Confédérés, Brabant, Hainaut-Hollande, Gueldre, Holstein, Suède.

## 3. Premiers pas

À la première partie, le **tutoriel** vous guide sur une quinzaine d'étapes (sélection de l'armée royale,
marche, province et ville, construction, recherche, diplomatie, fin de tour, rapport de saison, chronique,
impôt, gouverneur), avec des conseils propres à votre faction. « Passer l'étape » ou « Passer le tutoriel »
à tout moment ; Menu → Tutoriel le relance ; il se désactive dans Réglages → Partie.

Trois aides sont toujours à portée :
- **F1** : aide des commandes et principes ;
- **L** : encyclopédie (unités, bâtiments, technologies, ressources, traits, compétences, factions, religion,
  mécaniques), avec recherche et liens ;
- **K** : codex historique (personnages, batailles, notions), aussi ouvert par les mots soulignés des
  infobulles.

Survolez n'importe quelle icône : les infobulles donnent coûts, effets, prérequis et raisons de refus.

## 4. La carte de campagne

- **Caméra** : W A S D (Z Q S D en AZERTY) ou flèches, bords de l'écran (F2 active ou coupe ce défilement),
  molette pour le zoom, Q / E pour la rotation.
- **Clic gauche** : sélectionner une armée, une province ou une colonie (une colonie sous le curseur est
  prioritaire sur sa province). **Clic droit** (armée sélectionnée) : l'armée marche aussitôt vers le point
  visé du sol, attaque l'armée ennemie visée, ou marche sur la colonie visée (voir § 5, « Mouvement libre
  des armées »). La **bulle** dorée au sol montre jusqu'où elle peut aller cette saison.
- **Entrée** ou bouton **Fin du tour** : votre tour est clos ; chaque faction IA joue ensuite le sien, l'une
  après l'autre, puis la saison s'écoule (économie, sièges, population, événements).
- **Échap** : désélectionner, puis menu pause (sauvegarder, charger, réglages, aide, menu principal).
- **Modes de carte** : M mécontentement, N diplomatie, R religion ; F12 capture d'écran.
- **Barre du haut** : trésor, revenu de la saison et prévision, date, recherche en cours, boutons
  Diplomatie (P), Chronique, Cour (C), Technologies (T), Objectifs (O), Menu.
- **Journal** (en bas à gauche) et **rapport de saison** en fin de tour : batailles, prises, déclarations,
  morts et naissances, constructions et recherches achevées ; un clic centre la caméra.
- **Alertes** (à droite) : armée ennemie à la frontière, siège, dette, recherche inactive, bâtiment
  terminé, décision de chronique en attente.

Un tour est une saison : l'hiver ralentit les marches et affame les armées en pays ennemi. Les armées
traversent la mer entre deux ports (une cogue marque la traversée) ; embarquer consomme tout le mouvement de
la saison, et débarquer en terre hostile coûte des hommes.

## 5. Provinces et colonies

Une province n'est pas réduite à sa ville principale : elle contient de 3 à 6 **colonies** prenables
séparément — une cité et, selon les lieux, des villes, châteaux, abbayes et villages —, chacune avec son
propriétaire, son contrôleur, sa garnison, ses bâtiments et, le cas échéant, son siège. Cliquez une province :
terrain, population, mécontentement, dévastation, et l'**onglet Colonies** (liste des colonies avec
contrôleur, garnison, siège). Cliquez une colonie pour ouvrir son propre panneau.

### Les cinq types de colonie

| Type | Rôle |
|---|---|
| **Cité** | chef-lieu de la province (une seule par province) : la plus forte garnison, tous les bâtiments, port si la province est côtière |
| **Ville** | ville murée ou bastide : commerce, guildes, garnison moyenne |
| **Château** | forteresse d'un châtelain : garnison réduite mais aguerrie (hommes d'armes, arbalétriers), résiste longtemps |
| **Abbaye** | abbaye ou commanderie fortifiée : recherche (scriptorium), garnison légère |
| **Village** | bourg ouvert, sans fortification ni garnison de départ : pris dès qu'une armée ennemie y entre |

Chaque colonie a un poids qui fixe sa part de l'impôt et du recrutement de la province.

### Contrôle, revenu et bonus de province complète

Le propriétaire et le contrôleur d'une province sont ceux de sa **cité** : prendre la cité prend la
province, même si une enclave (château ou ville d'un tiers) y subsiste. L'impôt de la province (population,
richesse, taux d'imposition) est réparti entre les contrôleurs de ses colonies au prorata de leur poids ;
une colonie assiégée ne rapporte rien ce tour. Tenir **toutes** les colonies d'une province donne un bonus de
province complète : +10 % de revenu et −5 de mécontentement par saison.

### Garnisons

Chaque colonie entretient sa propre garnison. La cité en paie 50 %, comme une capitale ; les autres colonies
moins, car milices urbaines, châtelains et gardes d'abbaye étaient soldés sur place : ville 15 %, château
25 %, abbaye 10 %, village sans garnison de départ. Une armée peut **laisser des unités en garnison** dans la
colonie où elle stationne, si elle la contrôle et qu'elle n'est pas assiégée ; chaque type plafonne le nombre
d'unités qu'on peut y loger — 8 dans une cité, 4 dans un château, 3 dans une ville, 2 dans une abbaye, 1 dans
un village — pour empêcher d'y cacher une armée entière à moindre coût.

### Bâtiments et économie

Chaque type de colonie n'accepte que certains bâtiments : marché et halle des guildes en cité et en ville,
fortifications en cité, ville et château, scriptorium en cité et en abbaye ; les villages n'ont ni
fortification ni bâtiment d'institution. La couronne ne paie qu'une part de l'entretien des bâtiments hors de
la cité (droit de murage, châtellenie, temporel du monastère) : cité 100 %, château 50 %, village 50 %, ville
40 %, abbaye 25 %. Ressources (blé, vin, laine, sel, fer, bois, pierre…) et population par classe (paysans,
bourgeois, clergé, noblesse), avec leurs quatre jauges — mécontentement, santé, richesse, biens — restent
attachées à la province, pas à chaque colonie. Construire coûte des livres, parfois des matériaux, et
plusieurs saisons ; une seule construction à la fois par colonie.

- **Panneau de faction** (clic sur l'écu de la barre) : revenus et dépenses, entretien de l'armée et des
  bâtiments, frais de cour, biens, **impôt** Bas / Normal / Haut (plus d'argent contre plus de colère).
- **Gouverneurs** : un personnage nommé à la tête d'une province y applique ses compétences (impôts,
  ordre public, population) ; elles s'appliquent à toute la province, colonies comprises.

Le mécontentement monte avec l'impôt, la dévastation, le manque de biens et de santé, l'occupation
étrangère et une religion différente ; au-delà de 75 pendant deux saisons la province se soulève, au-delà
de 90 elle passe aux rebelles. La **peste** (Peste noire à partir de 1347) et la **famine** frappent les
populations ; hôtels-Dieu et adductions d'eau les protègent.

### Mouvement libre des armées

Les armées se déplacent librement sur la terre, comme dans un Total War : une armée en campagne se tient en
un point quelconque de la carte, ou dans une colonie (stationnée, en garnison ou assiégeant la place).

- **Bulle de portée** : sélectionnez une armée ; une zone dorée au sol, dont le voile s'épaissit vers le
  bord, montre les lieux qu'elle peut atteindre avec les points de mouvement qui lui restent cette saison.
- **Chemin en deux couleurs** : en survolant le sol, le chemin s'affiche en **vert** pour la part parcourue
  cette saison et en **rouge** pour les saisons suivantes, avec un jalon à chaque fin de tour ; l'étiquette
  donne le nombre de tours et le coût. L'aperçu ne tient pas compte des zones de contrôle : la marche réelle
  peut s'arrêter plus tôt.
- **Clic droit sur le sol** : l'armée part aussitôt, avance case par case et s'arrête au point visé ou quand
  ses points sont épuisés. Elle peut repartir dans la même saison s'il lui en reste.
- **Trajets sur plusieurs tours** : une cible hors de portée est gardée en mémoire ; au début de chaque tour,
  l'armée reprend d'elle-même sa marche, sauf nouvel ordre.
- **Portée** : environ 210 km de plaine par saison au printemps, en été et en automne, 140 km en hiver ; une
  route ancienne coûte un quart de moins. Collines, forêts, marais et montagnes coûtent de 1,5 à 3 fois une
  plaine ; les pentes trop raides sont infranchissables. De Paris, Orléans, Rouen ou Reims sont à une saison,
  Lyon à deux, Bordeaux ou Toulouse à trois. Compétences, technologies et traits du général allongent la
  marche ; un train de siège la ralentit.
- **Zone de contrôle** : chaque armée tient un cercle de 8 km autour d'elle (affiché en rouge au survol d'une
  armée ennemie). Une armée ennemie qui y entre s'arrête net.
- **Attaque** : armée sélectionnée, le curseur prend la forme de deux **épées croisées** au survol de ce
  qu'un clic droit attaquerait (armée ou place). Clic droit sur une armée ennemie à portée : votre armée
  marche au contact (à moins de 5 km) et la bataille est livrée aussitôt, tactique ou automatique à votre
  choix. Plus aucun mouvement ensuite.
- **Colonies** : clic droit sur une colonie ennemie pour l'**attaquer** : si l'armée l'atteint ce tour,
  elle met le siège et l'**assaut** est aussitôt proposé (bataille de siège, résolution automatique, ou
  « Maintenir le siège » pour affamer la place) ; sinon elle marche et l'assaut se donne au tour suivant
  (nouveau clic droit ou bouton « Donner l'assaut »). Une armée ennemie postée dans la place est attaquée
  aux portes. Un village sans garnison est pris sur-le-champ.
- **Faction en paix** : attaquer une armée ou une place d'une faction avec laquelle vous n'êtes pas en
  guerre ouvre une confirmation « Déclarer la guerre ? » qui détaille les conséquences (motif, réputation,
  prestige) ; confirmer déclare la guerre puis lance l'attaque. Alliés et vassaux ne s'attaquent pas ainsi.
- **Stationner** : sur une colonie amie, l'armée **stationne** : elle peut y laisser des unités en
  garnison et en ressortir dans la même saison si elle a encore des points.
- **Fleuves et gués** : les grands fleuves (Loire, Seine, Somme, Rhône, Saône, Garonne, Dordogne, Meuse,
  Escaut, Rhin, Tamise…) ne se franchissent qu'aux **ponts et gués** de 1337 (Orléans, Blois, Tours,
  Avignon, Pont-Saint-Esprit, London Bridge, Blanchetaque sur la Somme…), aux villes bâties sur leur rive et
  là où une ancienne voie romaine passe près d'une ville. Les rivières secondaires ralentissent seulement.
  Le chemin fait donc parfois un détour jusqu'au pont le plus proche.
- **Cols** : Alpes et Pyrénées ne se passent qu'aux cols connus (Mont-Cenis, Montgenèvre, Grand-Saint-Bernard,
  Simplon, Saint-Gothard, Somport, Roncevaux, le Perthus…), au prix d'une marche de montagne.
- **Mer** : marchez jusqu'à un port, puis clic droit sur la colonie portuaire d'arrivée ; la traversée
  prend toute la saison et l'armée débarque au port visé.
- **Tour séquentiel** : vous jouez d'abord toutes vos armées, dans l'ordre que vous voulez ; chaque ordre
  s'exécute tout de suite. À la fin de votre tour, chaque faction IA joue à son tour, dans un ordre fixe :
  ses armées reprennent leurs marches, puis elle donne et exécute ses ordres un par un. Les batailles qu'une
  IA engage contre vous pendant son tour sont résolues automatiquement et racontées dans le rapport de
  saison.

### Repli après une défaite

Une armée battue se replie automatiquement, dans cet ordre :

1. vers la colonie amie (à soi ou à un allié) la plus proche, libre de toute armée ennemie, à 280 km de
   marche au plus, par un chemin qui ne traverse ni place ennemie ni zone de contrôle ennemie (celles du
   champ de bataille exceptées) ;
2. sinon elle recule de 15 km à l'opposé du vainqueur, en perdant 10 % de traînards — à condition de
   garder un **refuge** : une colonie que nul ennemi ne tient, à 140 km de marche au plus par le même genre
   de chemin, et de ne pas avoir été **écrasée** (42 % de ses hommes perdus ou plus dans la bataille) ;
3. sinon c'est la **débandade** : 50 % de pertes, puis les survivants rejoignent la place amie la plus
   proche à toute distance s'ils gardent au moins 30 % de leurs effectifs, sinon l'armée se disperse et son
   général s'échappe seul.

Une armée qui s'enfonce en terre ennemie, loin de ses places et entourée d'armées adverses, risque donc bien
plus la débandade qu'une armée battue près de chez elle.

### Sièges des places secondaires

Villes, châteaux et abbayes s'assiègent comme une cité (vivres, brèche, assaut ou résolution automatique),
mais avec une garnison et une fortification propres, en général plus faibles que celles d'une cité — les
places secondaires changent donc de main plus souvent. Un village sans garnison ne se siège pas : il est
pris dès l'arrivée d'une armée ennemie.

### Interface des colonies

- **Panneau de colonie** : garnison, bâtiments, recrutement, construction (comme l'ancien panneau de ville,
  mais par colonie).
- **Onglet Colonies** du panneau de province : liste des colonies avec contrôleur, garnison et siège.
- **Clic droit sur une colonie** : l'armée sélectionnée y marche aussitôt (stationnement, siège ou prise).
- **Paliers de zoom** : de loin, provinces coloriées par contrôleur (hachurées si partagées) ; à moyenne
  distance, icônes des colonies (type et couleur du contrôleur), noms des cités, routes principales ; en vue
  rapprochée (échelle du comté), maquettes de colonies par type, hameaux, routes, champs et forêts, et
  armées en figurines.

La cour et l'administration coûtent d'autant plus que le royaume est vaste et que le trésor dort : un trésor
supérieur à six saisons de revenu est rongé par les frais de cour. En dette, les troupes perdent du moral :
licenciez ou levez l'impôt.

**La table et la médecine.** Chaque province mange par défaut pain et potage ; son contrôleur peut lui
imposer un autre régime (poisson, viande, laitages…) selon les ressources accessibles, la côte et les
technologies : il coûte des livres mais améliore santé et contentement, avec les règles du Carême et de
l'hiver. La branche de technologies **Médecine**, le jardin des simples et l'apothicairerie renforcent la
résistance à la peste et la guérison des blessés.

### Liste des colonies (touche B)

La touche **B** (ou le bouton « Colonies » de la barre du haut) ouvre un accès rapide façon
Total War à toutes vos colonies, dans le même coin que « Mes unités » (ouvrir l'une ferme
l'autre). Le trésor et le revenu net du dernier tour sont rappelés en tête, avec trois filtres
exclusifs — colonies **libres** (sans chantier), colonies avec une **construction ou promotion**
possible (▲), colonies **en danger** (⚠) — et un tri des provinces par revenu, nom ou ordre
public.

Les provinces se déplient sous forme de lignes : nom, revenu, chantiers en cours sur le total de
colonies (⚒), jauge d'agitation et, si une révolte a commencé, le nombre de saisons restantes. Le
bouton « ⌖ » centre la caméra sur la province et ouvre son panneau. Sous chaque province
dépliée, une ligne par colonie donne son revenu, son chantier en cours (ou « libre »), le badge
▲ (plein si une promotion est possible, en contour pour une construction neuve), sa garnison et,
le cas échéant, un siège ou la mention « occupée ». Cliquer une colonie y porte la caméra et
ouvre son propre panneau — aucune construction ni aucun recrutement ne se fait depuis cette
liste. Les infobulles détaillent les constructions possibles (▲) et les raisons du danger (⚠).

## 6. Armées, ravitaillement, chevauchées

- **Recruter** dans le panneau d'une colonie que vous contrôlez, puis **former une armée** ; la levée prend
  une ou plusieurs saisons, plafonnée par le poids de la colonie et bornée par ses bâtiments. Chaque type
  d'unité puise dans une classe de la province : archers chez les paysans, hommes d'armes et chevaliers dans
  la noblesse, milices chez les bourgeois.
- **Général** : nommez un personnage depuis sa fiche ; ses compétences (Commandement) et ses traits pèsent
  sur les batailles, le moral et le ravitaillement. Une armée sans général se débande plus vite.
- **Ravitaillement** : les armées vivent sur le pays ; en territoire ennemi, ravagé ou l'hiver, elles
  perdent des hommes (attrition). Certains bâtiments et technologies améliorent l'approvisionnement.
- **Postures** (panneau d'armée) : Normale, **Siège** (investir la place ennemie), **Chevauchée** (piller :
  butin, dévastation et mécontentement chez l'ennemi).
- Les armées alliées présentes sur la même colonie combattent ensemble, sous le meilleur général.

## 7. Batailles

Quand vos armées rencontrent l'ennemi, le dialogue d'avant-bataille propose **Livrer bataille** (3D) ou la
**résolution automatique** (réglable par défaut dans Réglages). Le champ reprend le relief et la météo de la
province et de la saison : la pluie gêne arcs et arbalètes, le brouillard raccourcit les portées, la boue
et les gués ralentissent.

| Commande | Effet |
|---|---|
| Clic gauche, glisser, Maj | sélectionner, rectangle, ajouter |
| Clic droit | marcher ou attaquer ; double clic droit : au pas de course |
| Glisser-droit | tracer et orienter la ligne |
| F / G / H | formation (ligne, colonne, schiltron, coin) / tir à volonté / halte |
| Z X V B N | ordres du chef : cri de guerre, rallier, pied à terre, dresser les pavois, pas de quartier |
| Ctrl+1…9, 1…9 | enregistrer un groupe, le rappeler (deux fois : centrer la caméra) |
| Espace, + / − | pause (ordres possibles en pause), vitesse ×1 / ×2 / ×4 (ou boutons en bas à droite) |
| W A S D, Q / E, molette | caméra |
| F1, F12, Échap | aide, capture, menu |

Les cartes d'unités, rangées par avant-garde, bataille et arrière-garde, montrent effectif, moral,
fatigue et munitions ; la minicarte se clique pour déplacer la caméra. Les flancs (+50 %) et les arrières
(+100 %) sont vulnérables, les piques arrêtent la cavalerie, les archers plantent leurs pieux, un régiment
sans moral se débande, et la mort du général ébranle toute l'armée. La bataille est gagnée quand l'ennemi
est en déroute ou quitte le champ ; « Retraite générale » sauve ce qui peut l'être.

## 8. Sièges

Une armée en posture **Siège** investit une colonie ennemie (cité, ville, château ou abbaye) : chaque
saison, les **vivres** de la garnison baissent (famine puis capitulation) et les **engins** (trébuchets,
mangonneaux, bombardes) ouvrent une **brèche**. Le panneau d'armée donne l'état du siège et une estimation
des chances ; **Donner l'assaut** mène à une bataille de siège 3D (ou à la résolution automatique). La
garnison peut tenter une **sortie**. Un village, sans fortification ni garnison de départ, ne se siège pas :
il est pris dès l'arrivée d'une armée ennemie.

Dans la bataille de siège, l'enceinte suit les fortifications de la colonie assiégée (le niveau et le type
choisissent la maquette) : courtines, tours, porte, place centrale. Les défenseurs tirent depuis le chemin de
ronde ; l'assaillant dispose d'échelles (lentes et vulnérables), de tours de siège qui déposent l'infanterie
sur le rempart, d'un bélier contre la porte et de ses engins qui abattent des pans de mur. Les chevaliers
mettent pied à terre. La place tombe quand la garnison est en déroute ou que la place centrale est tenue
60 secondes.

## 9. Personnages et dynasties

La **Cour** (C) liste la famille régnante, les généraux et les gouverneurs. La **fiche personnage** montre
âge, traits, compétences et l'arbre à trois branches — **Commandement**, **Gouvernance**, **Cour** — où
dépenser les points gagnés par l'expérience (batailles, gouvernement). Les traits s'acquièrent par les
événements (blessures, bravoure, piété, folie…), certains s'excluent. Les personnages se marient, ont des
enfants, meurent : la succession suit la loi du royaume (salique en France, préférence masculine,
cognatique, élective pour l'Empire, les républiques et la papauté), avec **régence** pour un héritier
mineur. Une dynastie éteinte laisse place à une nouvelle maison. Un personnage capturé peut être libéré
contre rançon.

## 10. Diplomatie et religion

Le panneau **Diplomatie** (P) montre l'attitude de chaque faction et ses raisons (intérêts, menace,
parenté, religion, historique) ; avant d'envoyer une proposition, il indique si elle serait acceptée et
pourquoi.

- **Guerre** : avec un casus belli (prétention dynastique, provinces revendiquées) ou sans, au prix de la
  réputation ; rompre une trêve est un parjure. Vos alliés sont appelés aux armes.
- **Paix** : selon le score de guerre, avec cessions de provinces, tribut, et une trêve de cinq ans.
- **Alliances**, **embargos** (commerce), **vassalité** (tribut, loyauté, rébellion), **mariages** entre
  maisons (prétentions et unions personnelles).
- **Religion** : faveur du pape (piété, dons, médiation), excommunication, **Grand Schisme** de 1378 à 1417
  (choix d'obédience Rome ou Avignon), hérésies (Lollards, Hussites).

L'IA mène la guerre de Cent Ans en historienne : la prétention d'Édouard III relance la guerre à la fin des
trêves, l'Écosse reste l'alliée de la France, les princes des Pays-Bas penchent vers l'Angleterre, la
Bourgogne suit le plus fort.

## 11. Technologies

Deux arbres, **militaire** et **civil**, plus la **médecine** (T). Les points de recherche viennent de la
base du royaume, des universités, monastères et scriptoria, et du dirigeant. Chaque technologie a une date
historique : la rechercher plus de vingt ans en avance coûte 25 % de plus. Arc long, plates, bombardes,
compagnies d'ordonnance ; moulins, comptabilité en partie double, lettres de change, imprimerie…

## 12. Chronique

Quatre-vingt-dix événements historiques et aléatoires ponctuent le siècle : L'Écluse, Crécy, Calais, la
Peste noire, Poitiers, la Jacquerie, Brétigny, du Guesclin, le Grand Schisme, la folie de Charles VI,
Azincourt, Troyes, Jeanne d'Arc, Formigny, Castillon… Certains demandent une décision (bouton
**Chronique (n)**, deux tours pour choisir, sinon le choix est fait d'office) ; certains en entraînent
d'autres (Nicopolis → rançon de Jean de Nevers, Montereau → alliance anglo-bourguignonne → Troyes).
L'histoire peut diverger : un événement n'a lieu que si ses conditions sont réunies.

## 13. Victoire et fin de partie

Le panneau **Objectifs** (O) suit vos objectifs historiques. Les remplir tous avant l'échéance donne la
victoire ; perdre toutes ses terres, la défaite. À l'échéance, la campagne se termine avec un score.

## 14. Réglages et sauvegardes

- **Réglages** (menu de départ ou Échap) : plein écran, résolution, vsync, échelle d'interface, défilement
  par les bords, vitesse de caméra, sauvegarde automatique (fréquence), confirmation de fin de tour,
  rapport de saison, batailles 3D ou automatiques par défaut, tutoriel, volumes musique et effets.
- **Sauvegardes** : emplacements nommés avec vignette, faction, date de jeu et date réelle ; sauvegarde
  automatique tous les N tours sur trois emplacements tournants ; « Continuer » reprend la plus récente.

## 15. Raccourcis clavier (carte)

| Touche | Action |
|---|---|
| W A S D / flèches, Q / E, molette | caméra |
| F2 | défilement par les bords |
| Entrée | fin du tour |
| Échap | désélectionner, menu pause |
| C / T / P / O | cour / technologies / diplomatie / objectifs |
| M / N / R | modes mécontentement / diplomatie / religion |
| L / K | encyclopédie / codex |
| F1 / F12 | aide / capture d'écran |

## 16. Crédits

Voir `CREDITS.md` et l'écran Crédits : icônes game-icons.net (CC BY 3.0), relief ETOPO 2022 (NOAA), côtes
et rivières Natural Earth, moteur Godot, godot-rust. Modèles, écus, sons, musiques et illustration du menu
sont générés par les outils du projet.
