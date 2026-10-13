# Relief exagere : ce que font les autres jeux de strategie

Date : 2026-10-13. Recherche web (budget 0 $). Les forums Paradox (dev diaries) bloquent la lecture automatique ("Validating browser") : ces diaries ne sont connus que par les extraits de recherche. Chaque affirmation cite sa source ; **[F]** = fait source, **[I]** = inference de l'auteur. Les lacunes sont listees en fin de section 1.

## 1. Paradox

### Imperator: Rome (cas le plus proche de notre besoin)
- [F] Dev diary #2 : la carte est a deux niveaux, la "cite" comme plus petite unite et la "province" regroupant ~10-12 cites. Exemple de la Sicile : 23 cites et 4 zones de montagne **impassables**, ce qui rend les campagnes militaires "plus interessantes". https://forum.paradoxplaza.com/forum/threads/imperator-development-diary-2-4th-of-june-2018.1103329/post-24327577 (via extrait de recherche)
- [F] Reponse de developpeur citee sur Steam (2018) : certaines zones vides sont inachevees, d'autres, comme les Alpes, sont laissees inhabitees volontairement. https://steamcommunity.com/app/859580/discussions/0/4358996214438789472/?ctp=2
- [F] Un mod communautaire (1204 - A Broken World) remplace "un enorme impassable" par ~50 zones impassables nommees et ameliore le rendu des provinces voisines, surtout aux Alpes : le bloc unique d'impassable etait juge insuffisant. https://steamcommunity.com/sharedfiles/filedetails/changelog/1881249823?p=4 (mod, pas le jeu de base)
- [F] Un joueur d'EU5 cite Imperator comme modele : "les cols sont la ou sont les colonies", montagnes impassables. https://steamcommunity.com/app/3450310/discussions/0/604157049603919833/ (un seul message, aucune reponse dev)
- [I] Le patron Imperator = montagne impassable *nommee* + places habitees aux cols : le col devient un lieu de jeu, pas un trou dans un masque.

### Europa Universalis V (Project Caesar)
- [F] Aucun Tinto Talk trouve ne decrit des cols ou des montagnes infranchissables. Les montagnes restent franchissables mais couteuses : le pathfinding d'attrition pondere la penalite par le cout de mouvement de chaque case (Tinto Talks #104, 15/04/2026), les penalites de terrain deviennent multiplicatives (#105, 22/04/2026). https://forum.paradoxplaza.com/forum/developer-diary/tinto-talks-104-15th-of-april-2026.1917017/page-2 ; https://forum.paradoxplaza.com/forum/developer-diary/tinto-talks-105-22nd-of-april-2026.1918289/page-3 (extraits de recherche)
- [F] Des terrains "impassables en hiver" existent, sans drain de moral pour l'armee qui attend (#104). Meme source.
- [F] Plainte joueur (05/2025) : carte "paper-y", peu de verticalite, montagnes sans benefice. https://steamcommunity.com/app/3450310/discussions/0/604157049603919833/
- [F] Un submod "Rivers and Peaks" ajoute des types de topographie (caps) augmentant la difficulte de mouvement : le modding comble un manque. https://catalogue.smods.ru/?p=408981
- [I] EU5 est un contre-exemple utile : relief reel/doux + cout de mouvement donne une carte percue comme plate. C'est exactement notre symptome.

### Crusader Kings III
- [F] Dev diary #177 "A Fresh Coat of Paint" (Petter Lundh) : les heightmaps et masques de terrain d'origine etaient peints a la main, des mois de travail ; les montagnes d'origine venaient surtout d'une normal map en tuiles ; le rafraichissement utilise des fonctions deja moddables, avec options pour les changements couteux en performance. https://devtrackers.gg/crusader-kings/p/3c2f2816-dev-diary-177-a-fresh-coat-of-paint (extrait ; page forum non lisible)
- [F] Dev diary #2 "The Medieval Map" : portee globalement egale a CK2, passe de recherche sur chaque zone. https://forum.paradoxplaza.com/forum/threads/ck3-dev-diary-2-the-medieval-map.1274052/page-2
- [I] Meme chez Paradox, la heightmap est un asset *artistique* peint, pas un MNT brut ; les montagnes sont des motifs repetes, pas de la geographie fidele.

### Victoria 3
- [F] Dev diary #49 : la carte doit etre "interessante mais assez apaisante pour ne pas concurrencer l'interface en complexite" ; hierarchie visuelle de ce qui est important ; "papermap" lointaine avec nombreuses petites illustrations, vue comme survol du monde ; shaders de vagues peu couteux. https://www.paradoxinteractive.com/games/victoria-3/news/dev-diary-49-graphic-overview
- [F] Dev diary #70 (arbre techno, hors carte) : le principal probleme est la lisibilite au zoom maximal, "similaire a l'etiquetage de carte". https://www.paradoxinteractive.com/games/victoria-3/news/victoria-3-dev-diary-70-feature-game-jam-pt2
- [I] Deux regimes de rendu (3D proche, papermap loin) : c'est deja notre ADR 0124 (3D < 1200 / parchemin).

**Lacunes** : pas lu le texte integral des diaries EU4 #1, EU5 "locations", CK3 #177 ; rien trouve sur des "mountain passes" officiels d'EU5.

## 2. Autres jeux

### Total War (Rome II, Shogun 2, Warhammer) - sujet traite par un autre agent, notes minimales
- [F] Rome II : carte ~4x Shogun 2, de l'Iberie a l'Afghanistan ; une couche de terrain sous la campagne sert de "giant battle map" ; la heightmap determine montagnes et lacs, la vegetation est auto-generee. https://gamesbeat.com/total-war-rome-ii-interview/
- [F] PC Gamer : carte stylisee a la topographie exageree ; construite autour des postures d'armee pour dominer de grandes regions par des chokepoints. Game Informer credite la creation de chokepoints recompensant la manoeuvre. https://www.pcgamer.com/total-war-rome-2-review/ ; https://gameinformer.com/games/total_war_rome_ii/b/pc/archive/2013/09/04/total-war-rome-2-review (extraits de recherche ; page PC Gamer non lisible)
- [F] Un fil TWCenter decrit le choix "montagnes exagerees, cote generalisee" comme proche de Shogun 2 (opinion de forum, non developpeur). https://www.twcenter.net/threads/alternative-map/603815/page-2
- [F] Le DLC Caesar in Gaul est juge le plus "Shogun II" en geographie de chokepoints. https://www.matchstickeyes.com/2015/02/22/total-war-rome-ii-dlc-campaigns-the-buyers-guide/
- [F] Warhammer III : echelle "sans precedent", ~2x la precedente ; passages nommes (Worm Pass) et ponts visibles dans les previews. https://www.pcgamesn.com/total-war-warhammer-3/map-size

### Mount & Blade II: Bannerlord
- [F] Carte de campagne reconstruite dans l'editeur de scenes ; colonies placees directement dans l'editeur. https://www.gamebanshee.com/k48cf
- [F] Les types de terrain servent a "controler le comportement de l'IA" (ex. zones ou certains bandits peuvent roder), modifient la vitesse des groupes et relient la carte aux scenes de bataille. Meme source.
- [F] Une recherche rapporte qu'un blog dev dit que la plus grande carte augmente le nombre de chokepoints (cols etroits dans les chaines, embuscades, patrouilles) - non verifie a la source (taleworlds.com bloque en 403). https://taleworlds.com/en/Games/Bannerlord/Blog/65
- [F] Vitesse de groupe : terrain et jour/nuit pris en compte. https://gamebanshee.com/k4ckd
- [I] Bannerlord separe "relief visuel" et "couche semantique de terrain" (polygones de types) : modele a copier pour nos passes.

### Humankind (Amplitude)
- [F] Objectif terrain : "pas necessairement realiste, mais assez plausible tout en etant stylise et coherent" (M. Gregoire) ; reliefs plaine / collines / montagnes "clairement distincts" (T. Raulin) ; collines trop raides imposent un detour ; ligne de vue bloquee par le relief. https://www.gamewatcher.com/humankind-terrain-second-developer-diary
- [F] Continents decoupes en territoires, biome selon altitude et latitude. https://www.pcgamer.com/uk/civ-competitor-humankind-makes-your-choice-of-terrain-vital-to-survival

### Civilization VI
- [F] Directeur artistique B. Busatti : on recule beaucoup la camera, "il faut que ca reste lisible a cette distance" ; les silhouettes differentes aident. Fog of war redessine en carte ancienne. https://www.gamedeveloper.com/art/making-i-civilization-vi-i-both-playable-and-beautiful ; https://twinfinite.net/2016/07/civilization-vi-devs-explain-the-games-art-style/
- [F] Retours joueurs : problemes de lisibilite des collines/terrain explore non visible (forum, pas dev). https://forum.quartertothree.com/t/civilization-vi/78555?page=187

### Field of Glory Kingdoms, Knights of Honor II, Manor Lords
- Rien d'exploitable trouve sur la conception de carte/cols. [F] seul fait : FoG Kingdoms etend la carte vers l'est/sud en suivant les routes commerciales historiques (Route de la soie). https://www.slitherine.com/game/field-of-glory-kingdoms-rajas-and-tribes
- Manor Lords : hors sujet (pas de carte strategique).

## 3. Ecrits de design sur les cartes strategiques

- [F] Mandate Order (retour de design publie sur Game Developer) : la carte est "la premiere question strategique" ; montagnes, cols et terrain ouvert fixent le cout de chaque decision ; si les joueurs ne lisent pas la carte, leurs choix perdent leur sens ; des limites ambigues entre montagne/prairie/col ont cree du tatonnement ; correctifs : cols plus visibles, bords de montagne clarifies. https://www.gamedeveloper.com/press-release/why-a-map-can-decide-what-kind-of-state-you-build (communique d'un petit studio : retour d'experience, pas etude)
- [F] Un chokepoint est une zone difficile a franchir et facile a defendre ; les cartes riches en chokepoints encouragent un style de jeu ; courant : s'etendre par plusieurs chokepoints puis fortifier ceux des frontieres. https://springrts.com/wiki/Chokepoint
- [F] Guide d'editeur Warzone : placer les passerelles aux passages etroits entre montagnes, d'au moins 4 cases de large. https://warzone.atlassian.net/wiki/x/CABNAQ
- [F] Recherche IA (AIIDE) : decomposition de cartes RTS en regions + chokepoints ; l'algorithme retrouve 97 % des chokepoints identifies par des humains. https://ojs.aaai.org/index.php/AIIDE/article/view/12405
- [F] Trope "Chokepoint Geography" : cols et ponts comme barrieres classiques, historiquement fortifies. https://www.tvtropes.org/pmwiki/pmwiki.php/Main/ChokepointGeography
- Non trouve : talk GDC ou article Gamasutra dedie a la distorsion de geographie reelle ou au rythme distance/tours. Ces points ci-dessous sont des inferences.

## 4. Synthese

| Technique | Jeux | Statut |
|---|---|---|
| Montagne = zone infranchissable nommee, vie aux cols | Imperator | [F] |
| Relief doux + cout de mouvement seul : carte percue plate | EU5 | [F] plainte joueur, [I] cause |
| Heightmap peinte a la main / stylisee | CK3, Humankind, Total War | [F] |
| Couche semantique de terrain separee du rendu | Bannerlord, Rome II | [F] |
| Chokepoints + postures comme structure strategique | Rome II | [F] |
| Silhouettes distinctes, lisibilite en dezoom | Civ VI, Humankind | [F] |
| Rendu double proche/lointain | Vic3 | [F] |

## Lecons pour Cent Ans

Toutes sont des **inferences** appuyees par les sources ci-dessus ; elles ne sont pas des faits sur nos donnees.

1. **Separer le relief visuel et la couche de jeu.** Garder une couche `data/` de "terrain strategique" (types : plaine, colline, montagne infranchissable, col, marais) distincte de la heightmap affichee (Bannerlord, Rome II). La regle de franchissabilite vit dans `core/`, le relief 3D ne fait que la montrer.
2. **Montagne infranchissable = polygones nommes, pas un seuil d'altitude.** Imperator et son mod (1 bloc -> ~50 zones nommees) montrent qu'un gros bloc anonyme est decevant. Nommer chaque massif (Pyrenees, Alpes, Massif central partiel, Carpates...) dans les donnees et l'UI.
3. **Les cols sont des lieux, pas des trous.** Pour chaque massif, une liste explicite de cols (Somport, Roncevaux, Mont-Cenis, Brenner, Saint-Bernard...) avec position, nom, saison eventuelle ; y mettre fort, peage, village (modele "les colonies sont aux cols", Imperator). Le franchissement doit etre un choix strategique (siege, embuscade, controle).
4. **Cols saisonniers.** EU5 a deja des terrains impassables en hiver ; cela donne du rythme (campagnes d'ete) sans nouveau systeme. Prevoir un drapeau `closed_seasons` par col.
5. **Ne pas compter sur un cout de mouvement seul.** EU5 (cout + attrition) reste "plat" pour les joueurs ; garder un vrai mur pour la haute montagne, cout eleve pour la moyenne, afin que le relief change les trajets.
6. **Distorsion assumee mais documentee.** Total War generalise la cote et exagere les massifs ; Humankind vise "plausible, stylise, coherent". Fixer des regles : exagerer en largeur et hauteur seulement les barrieres qui ont un role militaire, conserver les positions des villes et la topologie (quelles regions se touchent, quels cols relient quoi). Consigner l'ecart maximal toleres (km) dans un ADR.
7. **Verifier la topologie, pas la forme.** Test automatique en Rust : le graphe de regions apres exageration doit avoir les memes composantes et des chemins entre voisins historiques passant par les cols attendus (idee issue de la detection de chokepoints, AIIDE). Compter aussi les chokepoints pour equilibrer.
8. **Largeur de col suffisante pour une armee et pour le clic.** Guide Warzone : >= 4 cases ; chez nous, une passe doit rester selectionnable au zoom strategique et franchissable par une armee en colonne ; viser une largeur en pixels d'ecran minimale a l'echelle de vue moyenne.
9. **Cols plus visibles que le reste.** Le retour Mandate Order : rendre les passes saillantes et clarifier les bords montagne/plaine. Teinte/trace de route et nom au zoom moyen ; bord des massifs net (pente raide, crete lisible) pour que "infranchissable" se lise sans infobulle.
10. **Silhouettes distinctes en dezoom.** Civ VI/Humankind : plaine, colline, montagne doivent differer par la forme, pas la couleur. Notre vue parchemin (ADR 0124) doit reprendre les memes massifs, par hachures ou pictos stylises, pas par un rendu 3D reduit.
11. **Une seule verite entre 3D et parchemin.** Les deux rendus (Vic3 : 3D + papermap) doivent lire la meme couche semantique, sinon un col visible en 3D disparait au parchemin.
12. **Peindre la heightmap, ne pas seulement exagerer le MNT.** CK3 peint ses heightmaps ; nos x4.3 uniformes donnent une carte "terne". Pipeline propose : MNT reel -> masque de massifs -> renforcement local (crete, courbe de contraste) -> retouches a la main pour les cols, avec le masque de passes comme source.
13. **Les chokepoints donnent le sens des theatres d'operations.** Rome II construit la carte autour des chokepoints et postures. Verifier que chaque theatre (France du nord, Guyenne, Flandre, Italie, Iberie) a 2-4 entrees defendables, pas 0 ni 20 ; le bilan se mesure avec les seed bots (ADR 0348) : taux de passage par col, parties bloquees.
14. **Couche terrain -> IA et bataille.** Bannerlord lie les types de terrain au comportement de l'IA et aux scenes de bataille : le polygone "col" doit produire une bataille en defile, et l'IA doit savoir evaluer un col (garnison, blocus).
15. **Rythme distance/tours.** Aucune source trouvee ; hypothese a tester avec les seed bots : un massif infranchissable allonge les trajets d'un facteur mesurable ; fixer une cible (ex. traversee d'un massif = 2-4 tours de plus) et la verifier avant de valider la distorsion.
