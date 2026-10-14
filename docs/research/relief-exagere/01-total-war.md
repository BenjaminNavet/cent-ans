# Cartes de campagne Total War : ce qu'on sait réellement (recherche du 13/10/2026)

Légende : **[F]** fait tiré d'une source citée ; **[C]** affirmation de joueur/commentateur (opinion) ; **[I]** inférence de l'auteur, non sourcée.

## 0. Avertissement sur la qualité des sources

Budget 0 $, web ouvert uniquement. Creative Assembly (CA) n'a quasiment **jamais publié** de talk GDC/Develop détaillé sur le terrain de ses cartes de campagne, et les pages TWCenter, Reddit, PCGamer, Fandom, GamesBeat ont répondu 403/402/429 ou tronquées au moment de la recherche. Les faits solides sont donc peu nombreux ; le reste de ce document sépare soigneusement fait et inférence. Pistes non vérifiées à creuser plus tard (hors budget de cette passe) : vidéos « dev walkthrough » de la carte WH1 (16 min, mentionnée par The Sixth Axis), wiki officiel TW (`wiki.totalwar.com`), Mod Summit 2015 (TWCenter), ArtStation des artistes CA.

## 1. Pipeline de terrain

### 1.1 Outils : Terry et l'Assembly Kit
- **[F]** Terry est l'éditeur de l'Assembly Kit pour l'esthétique de la carte de campagne : height map, éclairage, props. Il donne accès aux « Terrain Raw Data » (~60 Go) dans le kit Attila. Sources : https://stevivor.com/news/official-mod-tools-now-available-for-total-war-attila ; https://pcgamer.com/total-war-attilas-modding-tools-released ; guide Warhammer : https://www.totalwar.com/?p=1105
- **[F]** Une refonte complète de la carte de campagne n'était pas possible aux modders en 2015 (rapport du Mod Summit 2015) : https://www.twcenter.net/threads/mod-summit-2015-report.679000/
- **[F]** Le wiki officiel documente le terrain **de bataille** de Terry : tuiles construites à partir de `height_map.png`, `blend_map.tif`, modèles rigides (arbres, rochers, bâtiments), `lf_heights.tif` pour la grande échelle sur plusieurs tuiles, `tile_map.png` + base XML pour placer les tuiles, `climate_map.png`, et `nogo.dds` qui interdit des zones aux batailles. Source : https://wiki.totalwar.com/w/Total_War:_ATTILA_Terry . Cette page ne décrit **pas** la carte de campagne ni les données de pathfinding.
- **[I]** Le flux est donc : image de hauteur (raster 2D) + masques peints (mélange de textures, climat) + props posés à la main, édités dans un outil maison, le tout compilé en « data packs ». Aucune source trouvée sur l'usage de World Machine ou de DEM réels pour la campagne WH.

### 1.2 Réel ou sculpté ?
- **[F]** Thrones of Britannia (2018) : CA dit « pouvoir travailler avec des données de terrain réelles » pour batailles et campagne ; carte 23x plus grande que les îles britanniques d'Attila (≈ la taille de toute la carte d'Attila) ; consultation d'un historien (Dr Neil McGuigan, St Andrews) pour le nord. Sources : https://pcgamesn.com/a-total-war-saga-thrones-of-britannia/total-war-saga-thrones-of-britannia-map-size ; https://gamewatcher.com/news/2017-11-12-the-total-war-saga-thrones-of-britannia-the-map-is-23x-bigger-than-total-war-attila (résumé de presse, pas le billet CA original).
- **[F]** Les mineures de ToB reçoivent des bâtiments selon la géographie locale (ferme, mine de fer, abbaye) : même source PCGamesN.
- **[I]** ToB est le cas le plus proche de Cent Ans (Europe médiévale, données réelles) ; CA y part probablement d'un DEM puis retouche à la main, mais **aucune source ne donne l'exagération verticale ni le retraitement**. Warhammer est une carte fantasy, donc forcément dessinée à la main.
- **[F]** Medieval II : aucune source technique trouvée dans cette passe. **[I]** (connaissance générale, non vérifiée ici) sa campagne était une carte 2D plate avec peu de relief lisible ; elle sert de contraste « Risk » : régions-polygones sur un fond peint, relief décoratif.

## 2. Échelle et distorsion

- **[F]** Rome II : « plus de régions que tout Total War précédent » ; presque 60 provinces et plus de 180 régions, de l'Afrique du Nord aux îles britanniques jusqu'à l'Afghanistan. Sources : https://gamereactor.eu/?p=16205 (hands-on, via recherche) ; https://pcgamesn.com/totalwar/total-war-rome-2-developer-diary-shows-multiple-objective-sieges-and-new-tactical-map
- **[F]** Un critique décrit la carte Rome II comme « exaggerated, poetic and three-dimensional recreation » de l'Europe/Moyen-Orient antiques : https://gulfnews.com/technology/gaming/rome-ii-game-doesnt-live-up-to-grandeur-1.1238678 . Cette critique juge le jeu trop étendu (« sprawls far too much to be a tight game »), pas la carte illisible.
- **[F]** Immortal Empires (WH3, 2022) : environ deux fois Mortal Empires ; ~554 régions annoncées ; ajout des « Sea Lanes » pour traverser la carte sans marcher tout le chemin. Sources : https://www.nme.com/news/total-war-warhammer-3-reveals-massive-map-for-immortal-empires-3253334 ; https://wccftech.com/total-war-warhammer-iii-immortal-empires-map-revealed/amp/
- **[I]** Ordres de grandeur pour Cent Ans : 443 provinces actuelles sur une carte Europe+Oural+Méditerranée est dans la même gamme que Rome II (≈ 180 régions) à WH3 (≈ 550). Un Rome II compresse bien plus de territoire par région que notre carte à l'échelle 1:1, ce qui est la source du « relief exagéré » ressenti : le relief y est lisible parce qu'une région fait quelques centimètres d'écran.
- **Non trouvé** : exagération verticale chiffrée, rapport km/unité. Ne pas citer de chiffre CA.

## 3. Terrain infranchissable, passes, goulets

- **[F]** Wiki WH (Fandom) : rivières, montagnes et forêts denses restreignent le déplacement sur la carte de campagne ; une armée peut utiliser sa zone de contrôle pour bloquer un passage étroit. Source (via recherche, page non récupérée directement ensuite) : https://totalwarwarhammer.fandom.com/wiki/Terrain
- **[F]** WH2 (mise à jour gratuite « forts ») : les forts sont des positions « hyper défendables, généralement dans des passes de montagne ou autres points clés » et donnent un avantage aux défenseurs aux goulets de la carte du monde. Sources : https://totalwar.com/?p=7980 (dev diary « Fort Fever », résumé de recherche) ; https://www.pcgamer.com/uk/this-free-total-war-warhammer-2-update-adds-enormous-unique-forts-to-defend-and-conquer
- **[C]** Joueurs : certains goulets de WH3 « ne sont pas programmés correctement » et WH2 avait plus de batailles de goulet ; la génération de carte de bataille depuis le terrain de campagne a été abandonnée après WH1 au profit de cartes préfabriquées. Source : https://steamcommunity.com/app/1142710/discussions/0/3731827719867915370 (opinion, pas CA).
- **[C]** Un joueur reproche à Shogun 2 d'avoir lancé la tendance des « petites avenues séparées par des montagnes massives » : peu de flanquement, IA prévisible, forêts = détours inutiles, îles ceintes de falaises. Un autre répond que cela donne moins de bugs, une IA qui fuit moins et plus compétitive (« I am playing total war, not total hide and go seek »). Source : https://steamcommunity.com/app/34330/discussions/0/4811511685067288873
- **[I]** Implication technique : les montagnes « murs » sont des zones de non-passage dans la grille de pathfinding de la campagne, avec des trouées délibérées (passes). Cela simplifie l'IA (graphe de couloirs) et crée des positions de blocage. Non confirmé par un document CA, mais cohérent avec les trois sources ci-dessus.

## 4. Lisibilité visuelle

- **[F]** Rome II : même sous le brouillard de guerre, on voit les pics de montagne, les contours des routes et des indices géographiques (review The Sixth Axis) : https://www.thesixthaxis.com/?p=194655
- **[F]** CA/WH1 : « la carte de campagne la plus détaillée et graphiquement impressionnante que nous ayons créée » (Ian Roxburgh, via The Sixth Axis) : https://www.thesixthaxis.com/2015/12/03/ca-walk-us-through-total-war-warhammers-greenskin-campaign-map/ . Aucun détail technique dans l'article.
- **[C]** Forum CA, 2024 : joueurs trouvent plusieurs régions WH ternes, sans « traits distinctifs, repères géographiques ou végétation unique » (hors Cathay, Darklands, Empire) ; ils demandent repères, skins de capitales, cycle jour/nuit, météo, signes visuels de corruption. Source : https://community.creative-assembly.com/total-war/total-war-warhammer/forums/8-general-discussion/threads/7019-now-is-the-time-to-spruce-up-the-campaign-map?page=1
- **[C]** Critique de Rome II centrée sur la lenteur et la perf (campagne à ~20 ips au lancement), pas sur la lisibilité du relief. Sources : https://gameinformer.com/games/total_war_rome_ii/b/pc/archive/2013/09/04/total-war-rome-2-review (résumé).
- **[I]** Ce qui rend ces cartes « lisibles » (non sourcé, à vérifier visuellement) : climat/biome peint par région avec couleurs franches, neige et roche sur les crêtes, caméra inclinée avec brouillard de distance, villes et armées en gros sprites. Chaque région est identifiable par sa teinte avant sa géométrie.

## 5. Manques de la recherche

Pas de source fiable pour : exagération verticale ; résolution de la heightmap de campagne ; format du masque de non-passage ; talk GDC/Develop de CA ; articles ArtStation de la carte WH ; analyse communautaire détaillée de Rome II (« trop comprimée »). Les recherches ciblées n'ont renvoyé que des previews de lancement. Reprendre via vidéos YouTube de dev walkthrough ou le wiki `wiki.totalwar.com` si l'on veut chiffrer.

## Leçons pour Cent Ans

1. **[F+I] Départ réel, arrivée stylisée.** ToB part de données de terrain réelles (source ci-dessus) : garder notre DEM comme base, mais le remodeler (voir 2 à 4) plutôt que de le rendre tel quel.
2. **[I] Découpler relief visuel et relief réel.** Une heightmap « artistique » (gain non linéaire sur les altitudes, crêtes accentuées, vallées élargies) doit s'ajouter au DEM ; le ×4,3 uniforme actuel agrandit aussi les collines sans caractère. Tester une courbe qui écrase les plaines et monte les massifs (Alpes, Pyrénées, Carpates, Massif central).
3. **[F+I] Montagnes = murs, passes = portes.** Dériver une carte de non-passage des pentes/altitudes (seuil), puis percer manuellement les passes historiques (Brenner, Saint-Gothard, Roncevaux/Somport, Mont-Cenis, Pas de Suse…). Le wiki WH confirme les montagnes comme restriction de mouvement et la zone de contrôle pour bloquer un passage.
4. **[F] Forts/châteaux aux passes.** WH2 place des forts aux passes de montagne ; chez nous : un type de lieu fortifié obligatoire sur chaque passe, avec bonus de défense et péage/blocage de la zone de contrôle.
5. **[C+I] Éviter l'effet « couloirs identiques ».** La critique Shogun 2 (avenues, peu de flanquement, IA prévisible) est un risque réel : garder 2 à 3 itinéraires par massif (dont un long et coûteux) et des détours par mer, pas un unique couloir.
6. **[I] Compter les passes comme arêtes de graphe.** Pour l'IA et le pathfinding Rust, représenter le non-passage dans la grille ET fournir un graphe de passes (nœuds = passes, coût = longueur + pente) pour les distances d'IA et les alertes de goulet.
7. **[F] Régions ≈ 180 à 550 comme cible de taille.** Rome II ≈ 180 régions, WH3 IE ≈ 554 : nos 443 sont dans la plage. Si le terrain est trop vaste pour être lisible, c'est la taille d'écran d'une région qui compte, pas le total.
8. **[F] Prévoir des voies rapides.** WH3 ajoute des Sea Lanes parce que la carte est grande : nos routes maritimes/Méditerranée jouent ce rôle ; assurer qu'elles contournent les murs de montagne.
9. **[C] Chaque région doit avoir une identité visuelle.** Les joueurs WH critiquent les régions ternes et réclament repères et végétation distincts : biomes déjà présents (TX/HB) à pousser en contraste ; repères géographiques lisibles (lac, fleuve, massif nommé).
10. **[I] Lisibilité par la couleur d'abord.** Neige/roche sur les crêtes, ombres de nuages déjà faites (RV) ; ajouter un éclairage rasant matinal et une ombre portée accentuée pour que le relief exagéré se lise depuis la caméra stratégique.
11. **[I] Garder le mode parchemin.** Notre vue lointaine parchemin (DV) correspond à la « vue stratégique » ; l'exagération ne concerne que la vue 3D < 1200.
12. **[F] Conserver le lien géographie→économie.** ToB attribue des bâtiments selon le terrain local : montagnes = mines, passes = péages ; renforce l'intérêt de la géographie exagérée (déjà amorcé par GR).
13. **[I] Itérer par prototype chiffré.** Aucun chiffre CA n'existe : prévoir un paramètre `relief_gain` / courbe et un `passability` testés sur 2-3 massifs avec captures (budget 5) plutôt que d'attendre une recette externe.
14. **[I] Ne pas imiter Medieval II.** La carte « Risk » plate est le contraste : elle est lisible mais jugée sans relief ; l'objectif est le relief comme décision stratégique, pas décor.
