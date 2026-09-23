# Provinces de la carte — situation au printemps 1337

Date : 2026-09-23. Jalon M1. 132 provinces dans `data/provinces/`, validées par `province.schema.json` (champ `geo` ajouté : `capital_lonlat`, `seed_lonlat`, `voronoi_weight`).

## 1. Principes

- **Emprise** : longitude -11° à 16°, latitude 35° à 60°. Toute terre de l'emprise appartient à une province (le diagramme de Voronoï pondéré ne laisse pas de trou), d'où quelques grandes provinces « de remplissage » aux marges (Scandinavie, Empire oriental, Italie du Sud) avec un poids élevé.
- **Granularité** : fine sur le théâtre de la guerre (France, Angleterre, Pays-Bas, Aquitaine), grossière ailleurs. Les grandes entités périphériques regroupent plusieurs pays historiques (ex. « Brandebourg et Mecklembourg », « Ulster et Connacht ») ; le nom d'affichage le signale.
- **Propriétaire (`owner`)** : la faction qui contrôle effectivement la province au printemps 1337, parmi les 15 factions existantes. Quand l'entité historique n'a pas de faction (Grenade, Venise, Florence, Naples, princes d'Empire, Hollande-Hainaut...), on prend la faction existante la plus proche et la `description` le précise ; la liste des factions à créer est en §3.
- **Suzerain (`overlord`)** : fiefs du roi de France (Guyenne, Ponthieu, Flandre, Bretagne, Bourgogne, Artois, Angoumois, Montpellier) et fiefs d'Empire (Franche-Comté, Savoie, Piémont, Milan, Gênes, Provence, Dauphiné, princes allemands). Naples est fief pontifical ; Majorque et Roussillon sont fiefs de l'Aragon.
- **`holder`** : uniquement des personnages existants dans `data/characters/`.
- **Population** : ordres de grandeur pré-peste (`uncertain: true`). Total sur la carte ≈ 41 millions. Bourgeois = part urbaine ; clergé 2,5 % ; noblesse 0,9 %. Repères : Paris ≈ 200 000, Milan et Venise ≈ 100 000, Florence 90 000, Gênes 60 000, Gand 60 000, Londres 50 000.
- **`geo.seed_lonlat`** : centre géographique de la province (pas la capitale) ; `voronoi_weight` 0,45 (Montpellier) à 1,8 (Bohême, Highlands, Naples, Sicile, Sardaigne, Götaland, Ulster). Les frontières seront retouchées en éditant seeds et poids.
- **`neighbors`** : calculés par le pipeline géo à partir des polygones ; seuls les six voisinages de l'échantillon M0 sont conservés.
- Vocabulaires libres introduits : cultures `cul_dutch`, `cul_irish`, `cul_basque`, `cul_galician`, `cul_andalusi`, `cul_venetian`, `cul_tuscan`, `cul_neapolitan`, `cul_sicilian`, `cul_danish`, `cul_swedish`, `cul_czech` ; zones maritimes `sea_atlantic`, `sea_irish_sea`, `sea_mediterranean`, `sea_adriatic`, `sea_baltic`.

## 2. Table des provinces

Colonnes : id, nom d'affichage, capitale, propriétaire, suzerain, tenant, notes (première phrase de la `description`).


### France du Nord (7)

| id | Nom | Capitale | Propriétaire | Suzerain | Tenant | Notes |
|---|---|---|---|---|---|---|
| `prov_artois` | Artois | Arras | `fac_burgundy` | `fac_france` | `chr_jeanne_iii_de_bourgogne` | Comté tenu par Jeanne III, épouse d'Eudes IV de Bourgogne, héritière de Mahaut (1329) et Jeanne II (1330). |
| `prov_boulonnais` | Boulonnais et Calaisis | Boulogne | `fac_france` | — | — | Comté de Boulogne, tenu par Robert VII d'Auvergne (faction à créer : maison d'Auvergne-Boulogne) en fief du roi. |
| `prov_ile_de_france` | Île-de-France | Paris | `fac_france` | — | — | Domaine royal autour de Paris : le Louvre, Notre-Dame, l'université, l'Hôtel-Dieu, Saint-Denis et le Lendit. |
| `prov_normandie` | Normandie (Rouen) | Rouen | `fac_france` | — | `chr_jean_de_normandie` | Haute-Normandie : ancien duché des Plantagenêts, réuni à la couronne en 1204, donné en apanage à Jean, fils du roi, en 1332. |
| `prov_normandie_ouest` | Normandie (Caen) | Caen | `fac_france` | — | `chr_jean_de_normandie` | Basse-Normandie : Bessin, Cotentin, Avranchin, bocage et herbages. |
| `prov_picardie` | Picardie et Vermandois | Amiens | `fac_france` | — | — | Amiénois, Vermandois (Saint-Quentin), Laonnois et Beauvaisis : domaine royal riche en blé et en draperies (waide d'Amiens). |
| `prov_ponthieu` | Ponthieu | Abbeville | `fac_england` | `fac_france` | — | Comté hérité par Édouard III de sa grand-mère Éléonore de Castille, tenu en fief du roi de France. |

### France de l'Ouest (5)

| id | Nom | Capitale | Propriétaire | Suzerain | Tenant | Notes |
|---|---|---|---|---|---|---|
| `prov_anjou` | Anjou | Angers | `fac_france` | — | — | Comté d'Anjou, apanage de Charles de Valois puis de Philippe VI avant son accession : entré au domaine en 1328. |
| `prov_bretagne` | Bretagne (Nantes et Rennes) | Nantes | `fac_brittany` | `fac_france` | `chr_jean_iii_de_bretagne` | Haute-Bretagne, pays gallo : Nantes (capitale ducale), Rennes, Saint-Malo, Dol et Penthièvre. |
| `prov_bretagne_ouest` | Basse-Bretagne | Quimper | `fac_brittany` | `fac_france` | `chr_jean_iii_de_bretagne` | Pays bretonnant : Léon, Cornouaille, Trégor, Vannetais. |
| `prov_maine` | Maine et Perche | Le Mans | `fac_france` | — | — | Comté du Maine (domaine royal avec l'Anjou) ; comtés d'Alençon et du Perche tenus par Charles II d'Alençon, frère du roi. |
| `prov_poitou` | Poitou | Poitiers | `fac_france` | — | — | Comté de Poitou, domaine royal depuis 1271 (apanage d'Alphonse de Poitiers). |

### France du Centre (7)

| id | Nom | Capitale | Propriétaire | Suzerain | Tenant | Notes |
|---|---|---|---|---|---|---|
| `prov_auvergne` | Auvergne | Clermont | `fac_france` | — | — | Terre royale d'Auvergne (Riom, bailliage royal) et comté d'Auvergne tenu par Guillaume XII (faction à créer : Auvergne-Boulogne). |
| `prov_berry` | Berry | Bourges | `fac_france` | — | — | Vicomté de Bourges royale depuis 1101 ; le duché de Berry ne sera créé qu'en 1360. |
| `prov_bourbonnais` | Bourbonnais et Marche | Moulins | `fac_france` | — | — | Duché de Bourbon (érigé en 1327 pour Louis Ier, petit-fils de saint Louis) et comté de la Marche, tenus en fief du roi par la maison de Bourbon (faction à créer). |
| `prov_limousin` | Limousin | Limoges | `fac_france` | — | — | Vicomté de Limoges disputée entre la maison de Bretagne (Jean III) et Jeanne de Penthièvre, sous suzeraineté française contestée par le duc d'Aquitaine. |
| `prov_nivernais` | Nivernais et Auxerrois | Nevers | `fac_flanders` | `fac_france` | `chr_louis_de_nevers` | Comté de Nevers (et Rethel) tenu par Louis Ier de Nevers, comte de Flandre. |
| `prov_orleanais` | Orléanais et Blésois | Orléans | `fac_france` | — | — | Domaine royal (duché d'Orléans créé en 1344 pour Philippe, frère du roi) et comté de Blois, tenu par Guy Ier de Châtillon, frère de Charles de Blois. |
| `prov_touraine` | Touraine | Tours | `fac_france` | — | — | Domaine royal depuis 1204 ; Tours, ville de Saint-Martin, est un grand centre de pèlerinage. |

### France de l'Est (5)

| id | Nom | Capitale | Propriétaire | Suzerain | Tenant | Notes |
|---|---|---|---|---|---|---|
| `prov_bar` | Barrois | Bar-le-Duc | `fac_france` | — | — | Comté de Bar tenu par Henri IV de Bar (faction à créer) : le Barrois mouvant, à l'ouest de la Meuse, est fief du roi de France depuis 1301 ; le reste relève de l'Empire. |
| `prov_bourgogne` | Bourgogne (duché) | Dijon | `fac_burgundy` | `fac_france` | `chr_eudes_iv` | Duché capétien, pairie de France, tenu par Eudes IV. |
| `prov_champagne` | Champagne | Troyes | `fac_france` | — | — | Comté de Champagne (Troyes, Reims, Châlons, Provins), réuni à la couronne en 1314 ; Jeanne II de Navarre y a renoncé en 1336 contre Angoulême et Mortain. |
| `prov_franche_comte` | Comté de Bourgogne | Dole | `fac_burgundy` | `fac_empire` | `chr_jeanne_iii_de_bourgogne` | Comté palatin de Bourgogne (Franche-Comté), terre d'Empire tenue par Jeanne III, épouse d'Eudes IV. |
| `prov_lyonnais` | Lyonnais et Forez | Lyon | `fac_france` | — | — | Lyon, rattachée au royaume en 1312 (traité de Vienne) ; comté de Forez (Guy VII, faction à créer) et sire de Beaujeu en fief du roi. |

### Aquitaine et Pyrénées (8)

| id | Nom | Capitale | Propriétaire | Suzerain | Tenant | Notes |
|---|---|---|---|---|---|---|
| `prov_agenais` | Agenais et Bazadais | Agen | `fac_france` | — | — | Agenais occupé par la France depuis la guerre de Saint-Sardos (1324) et non restitué en 1327 ; l'Angleterre en réclame la restitution. |
| `prov_angoumois` | Angoumois | Angoulême | `fac_navarre` | `fac_france` | `chr_jeanne_ii_de_navarre` | Comté d'Angoulême cédé en 1336 par Philippe VI à Jeanne II de Navarre et Philippe d'Évreux, en compensation de leurs droits sur la Champagne et la Brie. |
| `prov_bearn` | Béarn et Bigorre | Orthez | `fac_france` | — | — | Vicomté de Béarn (tenue en alleu selon Gaston II de Foix-Béarn, faction à créer) et comté de Foix, alliés du roi de France ; comté de Bigorre disputé, en garde royale. |
| `prov_gascogne` | Gascogne (Landes et Labourd) | Bayonne | `fac_england` | `fac_france` | — | Landes, Marsan, Tursan, Labourd : partie méridionale du duché anglais. |
| `prov_guyenne` | Guyenne | Bordeaux | `fac_england` | `fac_france` | — | Duché d'Aquitaine tenu par Édouard III en fief du roi de France (hommage de 1329 et 1331). |
| `prov_perigord` | Périgord | Périgueux | `fac_france` | — | — | Comté de Périgord tenu par Roger-Bernard de Périgord (maison de Talleyrand, faction à créer) en fief du roi de France, aux marches du duché d'Aquitaine anglais ; bastides et châteaux disputés. |
| `prov_quercy` | Quercy | Cahors | `fac_france` | — | — | Sénéchaussée royale de Quercy, revendiquée par l'Angleterre au titre du traité de Paris (1259). |
| `prov_saintonge` | Saintonge et Aunis | Saintes | `fac_france` | — | — | Saintonge au nord de la Charente et Aunis (La Rochelle), royales ; la Saintonge méridionale relève du duché d'Aquitaine anglais jusqu'à la confiscation de 1337. |

### Languedoc (5)

| id | Nom | Capitale | Propriétaire | Suzerain | Tenant | Notes |
|---|---|---|---|---|---|---|
| `prov_beaucaire` | Nîmois et Vivarais | Nîmes | `fac_france` | — | — | Sénéchaussée de Beaucaire-Nîmes, Vivarais, Gévaudan et Velay : rive droite du Rhône. |
| `prov_carcassonne` | Carcassès et Narbonnais | Carcassonne | `fac_france` | — | — | Sénéchaussée de Carcassonne et Béziers : Cité de Carcassonne, Narbonne (archevêché, port ensablé), Béziers. |
| `prov_montpellier` | Seigneurie de Montpellier | Montpellier | `fac_aragon` | `fac_france` | — | Seigneurie de Montpellier tenue par Jacques III, roi de Majorque (faction à créer), en fief du roi de France ; vendue à la France en 1349. |
| `prov_rouergue` | Rouergue et Armagnac | Rodez | `fac_france` | — | — | Comté de Rodez et comté d'Armagnac (Auch, Lectoure) tenus par Jean Ier d'Armagnac (faction à créer), lieutenant du roi de France en Languedoc. |
| `prov_toulousain` | Toulousain | Toulouse | `fac_france` | — | — | Sénéchaussée de Toulouse, domaine royal depuis 1271 (héritage d'Alphonse de Poitiers). |

### Provence, Dauphiné, Savoie (4)

| id | Nom | Capitale | Propriétaire | Suzerain | Tenant | Notes |
|---|---|---|---|---|---|---|
| `prov_comtat_venaissin` | Comtat Venaissin et Avignon | Avignon | `fac_papacy` | — | — | Comtat Venaissin, terre pontificale depuis 1274. |
| `prov_dauphine` | Dauphiné | Grenoble | `fac_empire` | `fac_empire` | — | Dauphiné de Viennois, terre d'Empire tenue par le dauphin Humbert II (maison de La Tour-du-Pin, faction à créer), en guerre contre la Savoie ; le transport à la France n'a lieu qu'en 1349. |
| `prov_provence` | Provence | Aix | `fac_empire` | `fac_empire` | — | Comté de Provence et de Forcalquier, terre d'Empire tenue par Robert d'Anjou, roi de Naples (faction angevine à créer ; fac_empire n'est ici que le suzerain nominal). |
| `prov_savoie` | Savoie | Chambéry | `fac_savoy` | `fac_empire` | `chr_aymon_de_savoie` | Comté de Savoie, Bresse, Bugey, Genevois et Valais (évêché de Sion) : passes du Mont-Cenis et du Grand-Saint-Bernard. |

### Pays-Bas (10)

| id | Nom | Capitale | Propriétaire | Suzerain | Tenant | Notes |
|---|---|---|---|---|---|---|
| `prov_brabant` | Brabant | Louvain | `fac_empire` | `fac_empire` | — | Duché de Brabant et Limbourg tenu par Jean III (faction à créer), qui louvoie entre France et Angleterre ; Louvain, Bruxelles, Malines et Anvers (étape de la laine anglaise en 1338). |
| `prov_flandre` | Flandre | Gand | `fac_flanders` | `fac_france` | `chr_louis_de_nevers` | Comté sous suzeraineté française, dominé par les trois « bonnes villes » drapières. |
| `prov_flandre_wallonne` | Flandre wallonne | Lille | `fac_france` | — | — | Lille, Douai et Orchies, détachées du comté de Flandre et rattachées au domaine royal par le traité d'Athis (1305) et le transport de Flandre (1312). |
| `prov_gueldre` | Gueldre | Arnhem | `fac_empire` | `fac_empire` | — | Comté de Gueldre (duché en 1339) et comté de Zutphen, tenus par Renaud II (faction à créer), marié à Éléonore, sœur d'Édouard III : allié anglais. |
| `prov_hainaut` | Hainaut | Mons | `fac_empire` | `fac_empire` | — | Comté de Hainaut tenu par Guillaume II d'Avesnes (faction Hainaut-Hollande à créer), beau-frère d'Édouard III ; Valenciennes est une place drapière. |
| `prov_holland` | Hollande et Zélande | Dordrecht | `fac_empire` | `fac_empire` | — | Comtés de Hollande et de Zélande tenus par Guillaume II d'Avesnes, comte de Hainaut (faction à créer). |
| `prov_liege` | Liège | Liège | `fac_empire` | `fac_empire` | — | Principauté épiscopale de Liège, évêque Adolphe de La Marck (faction à créer), allié de la France. |
| `prov_luxembourg` | Luxembourg | Luxembourg | `fac_empire` | `fac_empire` | — | Comté de Luxembourg tenu par Jean l'Aveugle, roi de Bohême (faction à créer ; allié de la France). |
| `prov_namur` | Namur | Namur | `fac_empire` | `fac_empire` | — | Comté de Namur tenu par Jean Ier de Namur, de la maison de Dampierre (faction à créer), cousin du comte de Flandre. |
| `prov_utrecht` | Utrecht | Utrecht | `fac_empire` | `fac_empire` | — | Principauté épiscopale d'Utrecht (Nedersticht et Oversticht : Deventer, Zwolle, Kampen), évêque Jean IV d'Arkel (faction à créer). |

### Empire : Rhin (6)

| id | Nom | Capitale | Propriétaire | Suzerain | Tenant | Notes |
|---|---|---|---|---|---|---|
| `prov_alsace` | Alsace | Strasbourg | `fac_empire` | `fac_empire` | — | Landgraviat de Haute-Alsace (Habsbourg), évêché et cité libre de Strasbourg, Décapole en gestation. |
| `prov_cologne` | Cologne | Cologne | `fac_empire` | `fac_empire` | — | Électorat de Cologne (archevêque Walram de Juliers) et ville libre de Cologne, la plus grande cité d'Allemagne ; comtés de Juliers (Guillaume, allié d'Édouard III, marquis en 1336), de Berg et de Clèves. |
| `prov_lorraine` | Lorraine | Nancy | `fac_empire` | `fac_empire` | — | Duché de Lorraine tenu par Raoul de Lorraine (faction à créer), fief d'Empire d'expression française ; Metz, Toul et Verdun sont des cités impériales et évêchés distincts. |
| `prov_mainz` | Mayence et Hesse | Mayence | `fac_empire` | `fac_empire` | — | Électorat de Mayence (archevêque Henri de Virnebourg), Francfort (cité impériale, foires) et landgraviat de Hesse (Henri II). |
| `prov_palatinate` | Palatinat du Rhin | Heidelberg | `fac_empire` | `fac_empire` | — | Comté palatin du Rhin, tenu par les Wittelsbach palatins (Rodolphe II et Robert Ier), neveux rivaux de l'empereur Louis IV (traité de Pavie, 1329). |
| `prov_trier` | Trèves | Trèves | `fac_empire` | `fac_empire` | — | Électorat de Trèves, archevêque Baudouin de Luxembourg, frère de l'empereur Henri VII et oncle de Jean de Bohême ; Coblence, confluent du Rhin et de la Moselle. |

### Empire : Sud et Alpes (6)

| id | Nom | Capitale | Propriétaire | Suzerain | Tenant | Notes |
|---|---|---|---|---|---|---|
| `prov_bern` | Berne | Berne | `fac_empire` | `fac_empire` | — | Cité impériale de Berne, en expansion contre la noblesse locale (Laupen, 1339) ; Fribourg (Habsbourg), pays de Vaud savoyard. |
| `prov_franconia` | Franconie | Nuremberg | `fac_empire` | `fac_empire` | — | Franconie : Nuremberg, cité impériale fidèle à Louis IV ; burgraves de Nuremberg (Hohenzollern) ; évêchés de Wurtzbourg et Bamberg. |
| `prov_oberbayern` | Haute-Bavière | Munich | `fac_empire` | — | `chr_ludwig_iv` | Duché de Haute-Bavière, domaine propre de l'empereur Louis IV de Wittelsbach, réuni à la Basse-Bavière depuis 1340. |
| `prov_swabia` | Souabe | Augsbourg | `fac_empire` | `fac_empire` | — | Souabe : comté de Wurtemberg (Ulrich III), Habsbourg antérieurs, cités impériales d'Ulm, Augsbourg, Constance, Zurich. |
| `prov_tirol` | Tyrol et Trente | Innsbruck | `fac_empire` | `fac_empire` | — | Comté de Tyrol tenu par Marguerite Maultasch et Jean-Henri de Luxembourg (répudié en 1341) ; évêchés de Brixen et de Trente. |
| `prov_waldstatten` | Waldstätten et Bâle | Lucerne | `fac_empire` | `fac_empire` | — | Confédération d'Uri, Schwyz, Unterwald et Lucerne (1332), en conflit avec les Habsbourg (Morgarten, 1315) ; évêché et cité de Bâle, évêché de Coire (Grisons). |

### Empire : Nord (4)

| id | Nom | Capitale | Propriétaire | Suzerain | Tenant | Notes |
|---|---|---|---|---|---|---|
| `prov_frisia` | Frise | Groningue | `fac_empire` | `fac_empire` | — | Frise libre : communautés paysannes sans seigneur (« liberté frisonne »), revendiquées par le comte de Hollande, qui y périra en 1345. |
| `prov_holstein` | Holstein et Lübeck | Lübeck | `fac_empire` | `fac_empire` | — | Comtés de Holstein (Gérard III de Rendsbourg, maître du Danemark) et villes libres de Lübeck (tête de la Hanse) et Hambourg. |
| `prov_lower_saxony` | Basse-Saxe | Brunswick | `fac_empire` | `fac_empire` | — | Duchés de Brunswick-Lunebourg (Welfs), archevêché de Brême, évêché de Hildesheim ; Brunswick et Lunebourg (salines) sont villes hanséatiques. |
| `prov_westphalia` | Westphalie | Münster | `fac_empire` | `fac_empire` | — | Duché de Westphalie (archevêque de Cologne), évêchés de Münster, Paderborn et Osnabrück, comté de La Marck (Adolphe II). |

### Empire : Est (4)

| id | Nom | Capitale | Propriétaire | Suzerain | Tenant | Notes |
|---|---|---|---|---|---|---|
| `prov_austria` | Autriche et Styrie | Linz | `fac_empire` | `fac_empire` | — | Duchés d'Autriche, de Styrie et de Carinthie (acquise en 1335) tenus par Albert II de Habsbourg (faction à créer). |
| `prov_bohemia` | Bohême | Prague | `fac_empire` | `fac_empire` | — | Royaume de Bohême de Jean de Luxembourg (faction à créer ; le roi combat pour la France), gouverné en son absence par son fils Charles, margrave de Moravie. |
| `prov_brandenburg` | Brandebourg et Mecklembourg | Berlin | `fac_empire` | — | `chr_ludwig_iv` | Marche de Brandebourg donnée en 1323 par Louis IV à son fils Louis le Brandebourgeois ; seigneuries de Mecklembourg et Poméranie occidentale (villes hanséatiques de Rostock, Wismar, Stralsund). |
| `prov_meissen` | Misnie et Thuringe | Meissen | `fac_empire` | `fac_empire` | — | Marche de Misnie et landgraviat de Thuringe tenus par Frédéric II de Wettin (faction à créer) ; Erfurt (Mayence), Leipzig (foires). |

### Scandinavie (3)

| id | Nom | Capitale | Propriétaire | Suzerain | Tenant | Notes |
|---|---|---|---|---|---|---|
| `prov_gotaland` | Götaland et Viken | Lödöse | `fac_empire` | — | — | Sud de la Norvège (Viken, Oslo) et Götaland occidental, sous Magnus IV Eriksson, roi de Suède et de Norvège (faction à créer ; fac_empire n'est qu'un substitut). |
| `prov_jutland` | Jutland | Ribe | `fac_empire` | — | — | Danemark sans roi (interrègne 1332-1340) : le Jutland est engagé au comte Gérard III de Holstein (faction Danemark/Holstein à créer). |
| `prov_sjaelland` | Seeland et Scanie | Roskilde | `fac_empire` | — | — | Seeland engagé au comte Jean III de Holstein ; la Scanie (Lund, foires au hareng de Skanör) s'est donnée en 1332 à Magnus IV de Suède. |

### Italie du Nord (7)

| id | Nom | Capitale | Propriétaire | Suzerain | Tenant | Notes |
|---|---|---|---|---|---|---|
| `prov_ferrara` | Ferrare et Mantoue | Ferrare | `fac_papacy` | — | — | Ferrare, fief pontifical tenu par les Este (Obizzo III, vicaire du pape depuis 1332 ; faction à créer) ; Mantoue des Gonzague (Luigi Ier, vicaire impérial). |
| `prov_genova` | Ligurie | Gênes | `fac_genoa` | `fac_empire` | — | République de Gênes déchirée entre guelfes et gibelins ; Savone rivale. |
| `prov_milano` | Milanais | Milan | `fac_milan` | `fac_empire` | `chr_azzone_visconti` | Seigneurie d'Azzone Visconti, vicaire impérial : Milan, Côme, Bergame, Crémone, Lodi, Pavie ; Brescia prise aux Scaligeri en 1337. |
| `prov_montferrat` | Montferrat | Casal | `fac_empire` | `fac_empire` | — | Marquisat de Montferrat tenu par Jean II Paléologue (faction à créer), en lutte avec Milan et les Angevins d'Asti pour Alexandrie et Asti. |
| `prov_piemont` | Piémont | Turin | `fac_savoy` | `fac_empire` | — | Terres savoyardes au-delà des Alpes : Turin, Pignerol, Suse, tenues par Philippe de Savoie-Achaïe pour le comte de Savoie ; marquisat de Saluces, vassal du Dauphin. |
| `prov_venezia` | Vénétie et Frioul | Venise | `fac_empire` | — | — | République de Venise (doge Francesco Dandolo ; faction à créer, indépendante de l'Empire malgré le substitut fac_empire), qui s'empare de Trévise et Padoue (1337) ; patriarcat d'Aquilée (Udine) au nord-est. |
| `prov_verona` | Vérone et Vicence | Vérone | `fac_empire` | `fac_empire` | — | Seigneurie de Mastino II della Scala (faction à créer), à son apogée (Vérone, Vicence, Padoue, Trévise, Parme, Lucques) mais attaquée depuis 1336 par la ligue Venise-Florence-Visconti ; Padoue perdue en août 1337. |

### Italie centrale (4)

| id | Nom | Capitale | Propriétaire | Suzerain | Tenant | Notes |
|---|---|---|---|---|---|---|
| `prov_ancona` | Marche d'Ancône et Ombrie | Ancône | `fac_papacy` | — | — | Marche d'Ancône et duché de Spolète (Pérouse, Assise) : terres pontificales gouvernées par des recteurs pendant l'exil avignonnais, disputées par les seigneurs locaux (Montefeltro, Malatesta). |
| `prov_bologna` | Bologne et Romagne | Bologne | `fac_papacy` | — | — | Bologne, terre d'Église sous la seigneurie de Taddeo Pepoli (1337) après la révolte contre le légat Bertrand du Pouget (1334) ; Romagne pontificale morcelée entre seigneurs (Polenta à Ravenne, Malatesta à Rimini). |
| `prov_firenze` | Toscane | Florence | `fac_empire` | — | — | République de Florence (faction à créer ; fac_empire n'est qu'un substitut) : compagnies des Bardi et Peruzzi, banquiers d'Édouard III, guerre contre Mastino della Scala pour Lucques ; républiques de Pise et de Sienne rivales. |
| `prov_lazio` | Latium et Patrimoine | Rome | `fac_papacy` | — | — | Rome sans pape, livrée aux luttes des Colonna et des Orsini ; Patrimoine de saint Pierre en Tuscia (Viterbe) et Campagne. |

### Italie du Sud et îles (4)

| id | Nom | Capitale | Propriétaire | Suzerain | Tenant | Notes |
|---|---|---|---|---|---|---|
| `prov_corsica` | Corse | Bonifacio | `fac_genoa` | — | — | Corse génoise (Bonifacio, Calvi), nominalement fief aragonais depuis 1297 ; seigneurs cinarchesi indociles dans l'intérieur. |
| `prov_napoli` | Royaume de Naples | Naples | `fac_empire` | `fac_papacy` | — | Royaume de Sicile citérieure (Naples), fief pontifical tenu par Robert d'Anjou (faction angevine à créer, également comte de Provence ; fac_empire n'est qu'un substitut) ; Abruzzes, Campanie, Pouille et Calabre. |
| `prov_sardegna` | Sardaigne | Cagliari | `fac_aragon` | — | — | Royaume de Sardaigne conquis sur Pise en 1323-1326 par l'infant Alphonse ; le judicat d'Arborea (Hugues II, à Oristano) reste vassal turbulent. |
| `prov_sicilia` | Sicile | Palerme | `fac_aragon` | — | — | Royaume de Trinacrie (Sicile) tenu par Frédéric III puis Pierre II d'Aragon-Sicile (branche cadette ; faction à créer, fac_aragon n'est qu'un substitut), en guerre intermittente contre Naples depuis les Vêpres siciliennes. |

### Ibérie : Nord (Castille) (5)

| id | Nom | Capitale | Propriétaire | Suzerain | Tenant | Notes |
|---|---|---|---|---|---|---|
| `prov_biscay` | Biscaye et Guipuscoa | Bilbao | `fac_castile` | — | — | Seigneurie de Biscaye (Juan Núñez de Lara, en révolte contre Alphonse XI jusqu'en 1336), Guipuscoa et Alava sous la couronne de Castille. |
| `prov_castilla_vieja` | Vieille-Castille | Burgos | `fac_castile` | — | — | Cœur du royaume de Castille : Burgos (laine de la Mesta exportée vers la Flandre), Valladolid (cour itinérante d'Alphonse XI), Ségovie, Soria. |
| `prov_galicia` | Galice | Saint-Jacques | `fac_castile` | — | — | Royaume de Galice sous la couronne de Castille ; pèlerinage de Saint-Jacques-de-Compostelle, archevêché puissant, pêche et cabotage. |
| `prov_leon` | León et Asturies | León | `fac_castile` | — | — | Royaume de León et principauté des Asturies (Oviedo), uni à la Castille depuis 1230. |
| `prov_navarra` | Navarre | Pampelune | `fac_navarre` | — | `chr_jeanne_ii_de_navarre` | Royaume de Navarre de Jeanne II et Philippe d'Évreux, indépendant depuis 1328 après le rejet des Valois ; Pampelune, Estella, Tudela ; cols de Roncevaux, chemin de Saint-Jacques. |

### Ibérie : Centre (Castille) (2)

| id | Nom | Capitale | Propriétaire | Suzerain | Tenant | Notes |
|---|---|---|---|---|---|---|
| `prov_extremadura` | Estrémadure | Badajoz | `fac_castile` | — | — | Estrémadure léonaise (Badajoz, Cáceres, Plasencia), frontière avec le Portugal en guerre (1336-1339) ; terres des ordres militaires d'Alcántara et de Santiago, pâturages de la Mesta. |
| `prov_toledo` | Nouvelle-Castille | Tolède | `fac_castile` | — | `chr_alfonso_xi` | Royaume de Tolède : Tolède (primat des Espagnes, capitale de jeu de la Castille), Madrid, Cuenca, Guadalajara. |

### Ibérie : Sud (Castille, Grenade) (5)

| id | Nom | Capitale | Propriétaire | Suzerain | Tenant | Notes |
|---|---|---|---|---|---|---|
| `prov_cordoba` | Andalousie (Cordoue et Jaén) | Cordoue | `fac_castile` | — | — | Royaumes de Cordoue et de Jaén, marche face à l'émirat de Grenade ; cuirs de Cordoue, oliviers, ordres militaires sur la frontière. |
| `prov_granada` | Grenade et Almería | Grenade | `fac_castile` | — | — | Émirat nasride de Grenade sous Yusuf Ier (faction musulmane à créer ; fac_castile n'est qu'un substitut, l'émirat étant tributaire de la Castille par intermittence). |
| `prov_malaga` | Málaga et Ronda | Málaga | `fac_castile` | — | — | Ouest de l'émirat de Grenade : Málaga, Ronda, Algésiras ; Gibraltar reprise par les Mérinides en 1333 et tenue avec Algésiras par le sultan Abu al-Hasan (faction à créer). |
| `prov_murcia` | Murcie | Murcie | `fac_castile` | — | — | Royaume de Murcie castillan (adelantado Pedro López de Ayala), amputé d'Alicante et Orihuela cédées à l'Aragon en 1304. |
| `prov_sevilla` | Andalousie (Séville) | Séville | `fac_castile` | — | — | Séville, Cadix, Jerez : frontière maritime face aux Mérinides (Gibraltar perdu en 1333 ; la bataille du Salado aura lieu en octobre 1340). |

### Couronne d'Aragon (5)

| id | Nom | Capitale | Propriétaire | Suzerain | Tenant | Notes |
|---|---|---|---|---|---|---|
| `prov_aragon` | Aragon | Saragosse | `fac_aragon` | — | `chr_pedro_iv` | Royaume d'Aragon proprement dit (Saragosse, Huesca, Teruel), noblesse jalouse de ses fueros face à Pierre IV le Cérémonieux. |
| `prov_barcelona` | Catalogne | Barcelone | `fac_aragon` | — | `chr_pedro_iv` | Principauté de Catalogne : Barcelone, capitale de jeu de la couronne d'Aragon, ses corts et son Consolat de Mar ; draperies, flotte marchande, rivalité avec Gênes. |
| `prov_mallorca` | Majorque | Palma | `fac_aragon` | `fac_aragon` | — | Royaume de Majorque (Majorque, Minorque, Ibiza) de Jacques III (faction à créer), vassal de l'Aragon ; il sera dépossédé par Pierre IV en 1343. |
| `prov_roussillon` | Roussillon et Cerdagne | Perpignan | `fac_aragon` | `fac_aragon` | — | Comtés de Roussillon et de Cerdagne, terres continentales du royaume de Majorque (Jacques III, faction à créer) ; Perpignan, sa capitale drapière. |
| `prov_valencia` | Valence | Valence | `fac_aragon` | — | — | Royaume de Valence, huerta irriguée par les mudéjars (majorité rurale musulmane), Alicante et Orihuela acquises en 1304. |

### Portugal (3)

| id | Nom | Capitale | Propriétaire | Suzerain | Tenant | Notes |
|---|---|---|---|---|---|---|
| `prov_alentejo` | Alentejo et Algarve | Évora | `fac_portugal` | — | `chr_afonso_iv` | Alentejo (Évora, Beja, Elvas face à Badajoz) et royaume de l'Algarve (Silves, Faro), conquis sur les Maures en 1249. |
| `prov_lisboa` | Estrémadure et Beira | Lisbonne | `fac_portugal` | — | `chr_afonso_iv` | Lisbonne, capitale d'Alphonse IV (en guerre contre la Castille 1336-1339), Coimbra (université), Santarém, Leiria. |
| `prov_porto` | Entre-Douro-et-Minho | Porto | `fac_portugal` | — | `chr_afonso_iv` | Nord du Portugal : Porto, Braga (primat), Guimarães, Trás-os-Montes. |

### Angleterre du Sud (6)

| id | Nom | Capitale | Propriétaire | Suzerain | Tenant | Notes |
|---|---|---|---|---|---|---|
| `prov_cornwall` | Cornouailles | Launceston | `fac_england` | — | `chr_edward_of_woodstock` | Duché de Cornouailles créé le 17 mars 1337 pour Édouard de Woodstock, fils aîné du roi ; stannaries (étain), pêche, langue cornique encore vivante. |
| `prov_devon` | Devon et Somerset | Exeter | `fac_england` | — | — | Devon et Somerset : Exeter (comte Hugh Courtenay), Dartmouth et Plymouth (ports d'embarquement pour la Gascogne), Wells et Glastonbury. |
| `prov_kent` | Kent | Cantorbéry | `fac_england` | — | — | Porte de l'Angleterre face au continent : les Cinque Ports (Douvres, Sandwich, Hythe, Romney) fournissent la flotte royale ; Cantorbéry est le siège du primat. |
| `prov_middlesex` | Londres, Middlesex et Essex | Londres | `fac_england` | — | `chr_edward_iii` | Cité de Londres (capitale de jeu de l'Angleterre), Westminster (Parlement, Échiquier), Tour de Londres ; Middlesex, Essex et Hertfordshire. |
| `prov_sussex` | Sussex et Surrey | Chichester | `fac_england` | — | — | Sussex (rapes de Lewes, Arundel, Pevensey) et Surrey : forêt du Weald, forges, Cinque Ports de Hastings, Rye et Winchelsea. |
| `prov_wessex` | Wessex (Hampshire, Wiltshire, Dorset) | Winchester | `fac_england` | — | — | Hampshire, Wiltshire, Dorset et Berkshire : Winchester (étape de la laine), Southampton (port des vins de Gascogne, brûlé par les Franco-Génois en 1338), Salisbury. |

### Angleterre de l'Est (1)

| id | Nom | Capitale | Propriétaire | Suzerain | Tenant | Notes |
|---|---|---|---|---|---|---|
| `prov_norfolk` | Est-Anglie et Lincoln | Norwich | `fac_england` | — | — | Norfolk, Suffolk, Cambridgeshire et Lincolnshire : la région la plus peuplée d'Angleterre. |

### Angleterre du Centre (3)

| id | Nom | Capitale | Propriétaire | Suzerain | Tenant | Notes |
|---|---|---|---|---|---|---|
| `prov_gloucester` | Gloucester et Bristol | Bristol | `fac_england` | — | — | Gloucestershire, Worcestershire, Herefordshire, Shropshire : Bristol (deuxième port du royaume, vins de Gascogne), marches galloises des Mortimer et des Bohun, forêt de Dean (fer). |
| `prov_oxford` | Vallée de la Tamise (Oxford) | Oxford | `fac_england` | — | — | Oxfordshire, Buckinghamshire, Bedfordshire, Northamptonshire : université d'Oxford, laine des Cotswolds, château de Windsor. |
| `prov_warwick` | Mercie (Warwick et Stafford) | Coventry | `fac_england` | — | — | Warwickshire, Staffordshire, Leicestershire, Derbyshire, Nottinghamshire : Coventry (draps bleus), forêt d'Arden, Sherwood, plomb du Peak. |

### Angleterre du Nord (3)

| id | Nom | Capitale | Propriétaire | Suzerain | Tenant | Notes |
|---|---|---|---|---|---|---|
| `prov_lancashire` | Lancastre et Cheshire | Lancastre | `fac_england` | — | `chr_henry_of_grosmont` | Comté palatin de Lancastre (Henri de Lancastre, père d'Henri de Grosmont) et comté palatin de Chester (domaine du prince). |
| `prov_northumberland` | Northumberland et Cumbrie | Newcastle | `fac_england` | — | — | Marches d'Écosse : Northumberland (Percy), palatinat de Durham, Cumberland et Westmorland (Carlisle). |
| `prov_yorkshire` | Yorkshire | York | `fac_england` | — | — | Yorkshire : York (archevêché, siège du gouvernement pendant les guerres d'Écosse), Hull (port royal de la laine), abbayes cisterciennes (Fountains, Rievaulx) et leurs troupeaux. |

### Pays de Galles (2)

| id | Nom | Capitale | Propriétaire | Suzerain | Tenant | Notes |
|---|---|---|---|---|---|---|
| `prov_deheubarth` | Galles du Sud et Marches | Carmarthen | `fac_england` | — | — | Carmarthen et Cardigan (principauté) et seigneuries des Marches : Glamorgan (Despenser), Pembroke (Hastings), Brecon (Bohun). |
| `prov_gwynedd` | Galles du Nord (Gwynedd) | Caernarfon | `fac_england` | — | — | Principauté de Galles (Gwynedd, Anglesey, Ceredigion) tenue par le roi depuis 1284, châteaux d'Édouard Ier (Caernarfon, Conwy, Harlech, Beaumaris) ; le titre de prince de Galles reste à conférer à Édouard de Woodstock (1343). |

### Écosse (5)

| id | Nom | Capitale | Propriétaire | Suzerain | Tenant | Notes |
|---|---|---|---|---|---|---|
| `prov_fife` | Fife et Perth | Saint Andrews | `fac_scotland` | — | — | Fife, Strathearn et Menteith : Saint Andrews (primat d'Écosse), Scone, Dunfermline. |
| `prov_galloway` | Galloway et Annandale | Dumfries | `fac_england` | — | `chr_edward_balliol` | Galloway, patrimoine des Balliol, et Annandale (Bruce) : base d'Édouard Balliol, roi d'Écosse sous protection anglaise (faction Balliol à créer ou rattachée à fac_england). |
| `prov_highlands` | Highlands et Moray | Inverness | `fac_scotland` | — | `chr_andrew_murray` | Moray, Ross, Argyll, Badenoch, Aberdeen et Angus : pays gaélique des clans, refuge du parti de David II sous le Gardien Andrew Murray (Bothwell, Avoch). |
| `prov_lothian` | Lothian et Marches | Édimbourg | `fac_england` | — | `chr_edward_balliol` | Lothian et Marches écossaises cédées à Édouard III par Édouard Balliol (traité de Newcastle, 1334) ; château d'Édimbourg tenu par une garnison anglaise (1335-1341). |
| `prov_renfrew` | Clydesdale et Renfrew | Renfrew | `fac_scotland` | — | `chr_robert_stewart` | Renfrew, Clydesdale, Lennox et Ayrshire : terres de Robert Stewart, Grand Sénéchal et Gardien, héritier présomptif. |

### Irlande (3)

| id | Nom | Capitale | Propriétaire | Suzerain | Tenant | Notes |
|---|---|---|---|---|---|---|
| `prov_dublin` | Pale et Leinster | Dublin | `fac_england` | — | — | Seigneurie d'Irlande : Dublin (justicier), Meath, Kildare (FitzGerald), Leinster anglo-normand ; les clans Ó Broin et Ó Tuathail harcèlent le Pale depuis les monts de Wicklow. |
| `prov_munster` | Munster | Cork | `fac_england` | — | — | Munster : comtes de Desmond (Maurice FitzGerald) et d'Ormond (James Butler, 1328 ; faction anglo-irlandaise à créer), villes royales de Cork et Limerick ; rois gaéliques Mac Carthaigh et Ó Briain dans le Kerry et le Thomond. |
| `prov_ulster` | Ulster et Connacht | Carrickfergus | `fac_england` | — | — | Comté d'Ulster en déshérence depuis le meurtre de Guillaume de Burgh (1333) ; Connacht aux Burke gaélisés et aux Ó Conchobhair ; Ó Néill de Tír Eoghain, Ó Domhnaill. |

## 3. Factions à créer dans un jalon ultérieur

Les provinces ci-dessous sont attribuées à une faction de substitution ; leur `description` le mentionne.

| Entité 1337 | Substitut actuel | Provinces concernées |
|---|---|---|
| Royaume de Naples / comté de Provence (Robert d'Anjou) | `fac_empire` | `prov_provence`, `prov_napoli` |
| Dauphiné de Viennois (Humbert II) | `fac_empire` | `prov_dauphine` |
| Hainaut-Hollande-Zélande (Guillaume II d'Avesnes) | `fac_empire` | `prov_hainaut`, `prov_holland` |
| Duché de Brabant (Jean III) | `fac_empire` | `prov_brabant` |
| Gueldre (Renaud II) | `fac_empire` | `prov_gueldre` |
| Principautés épiscopales de Liège et d'Utrecht | `fac_empire` | `prov_liege`, `prov_utrecht` |
| Comté de Namur (Jean Ier) | `fac_empire` | `prov_namur` |
| Luxembourg-Bohême (Jean l'Aveugle) | `fac_empire` | `prov_luxembourg`, `prov_bohemia` |
| Duché de Lorraine (Raoul) | `fac_empire` | `prov_lorraine` |
| Électorats de Cologne, Trèves, Mayence ; Palatinat ; Habsbourg (Autriche, Haute-Alsace) ; Wettin (Misnie) ; Welfs ; Holstein ; Wurtemberg ; Tyrol ; villes libres | `fac_empire` | `prov_cologne`, `prov_trier`, `prov_mainz`, `prov_palatinate`, `prov_austria`, `prov_alsace`, `prov_meissen`, `prov_lower_saxony`, `prov_holstein`, `prov_swabia`, `prov_tirol`, `prov_franconia`, `prov_westphalia` |
| Confédération des Waldstätten, Berne, Frise libre | `fac_empire` | `prov_waldstatten`, `prov_bern`, `prov_frisia` |
| Danemark en interrègne (comtes de Holstein) ; Suède-Norvège (Magnus IV) | `fac_empire` | `prov_jutland`, `prov_sjaelland`, `prov_gotaland` |
| République de Venise | `fac_empire` | `prov_venezia` |
| République de Florence (et Pise, Sienne) | `fac_empire` | `prov_firenze` |
| Scaligeri de Vérone | `fac_empire` | `prov_verona` |
| Montferrat (Paléologue) | `fac_empire` | `prov_montferrat` |
| Este de Ferrare, Gonzague, Pepoli de Bologne (vicaires pontificaux) | `fac_papacy` | `prov_ferrara`, `prov_bologna` |
| Royaume de Sicile (Aragon-Sicile) | `fac_aragon` | `prov_sicilia` |
| Royaume de Majorque (Jacques III) | `fac_aragon` | `prov_mallorca`, `prov_roussillon`, `prov_montpellier` |
| Émirat nasride de Grenade (Yusuf Ier) ; Mérinides à Gibraltar-Algésiras | `fac_castile` | `prov_granada`, `prov_malaga` |
| Grands vassaux français : Bourbon, Foix-Béarn, Armagnac, Périgord, Auvergne-Boulogne, Forez, Bar, Chalon-Auxerre, Blois | `fac_france` | `prov_bourbonnais`, `prov_bearn`, `prov_rouergue`, `prov_perigord`, `prov_auvergne`, `prov_boulonnais`, `prov_lyonnais`, `prov_bar`, `prov_nivernais`, `prov_orleanais` |
| Édouard Balliol (Écosse anglo-balliolienne) | `fac_england` | `prov_lothian`, `prov_galloway` |
| Seigneurs anglo-irlandais (Desmond, Ormond, Kildare) et rois gaéliques (Ó Néill, Ó Conchobhair, Ó Briain) | `fac_england` | `prov_munster`, `prov_ulster`, `prov_dublin` |
| Judicat d'Arborea | `fac_aragon` | `prov_sardegna` |

## 4. Choix discutables et incertitudes

- **Agenais** : occupé par la France depuis 1324 et non restitué en 1327 ; attribué à `fac_france` avec mécontentement élevé. La Saintonge est modélisée royale bien que sa partie sud fût anglaise jusqu'à la confiscation.
- **Limousin** : vicomté de Limoges tenue par la maison de Bretagne mais province attribuée à `fac_france` (sénéchaussée royale), faute de découpage plus fin.
- **Lothian** : Édimbourg et les Marches sont anglo-balliolliennes (1334-1341) alors que `fac_scotland` a `prov_lothian` pour capitale : le chargeur doit accepter une capitale de faction tenue par une autre faction (Écosse « en exil », cohérent avec David II à Château-Gaillard). Perth (`prov_fife`) et Stirling sont en réalité des garnisons anglaises isolées dans une province écossaise.
- **Avignon** : la ville appartient au comte de Provence jusqu'en 1348 mais est regroupée avec le Comtat sous `fac_papacy` (la Curie y réside).
- **Provence et Naples** sous `fac_empire` : choix très artificiel (Naples est fief pontifical, pas impérial) ; à corriger dès qu'une faction angevine existe.
- **Grenade** sous `fac_castile` : idem, seule solution sans faction musulmane ; religion `rel_islam` et culture `cul_andalusi` sont posées correctement.
- **Scandinavie, Bohême, Autriche, Italie du Sud** : provinces de remplissage aux marges de l'emprise, découpage grossier ; Vienne est hors carte (Autriche représentée par Linz).
- **Capitales de jeu** : Tolède (Castille), Barcelone (Aragon), Munich (Empire), Dole (Franche-Comté), Louvain (Brabant), Dordrecht (Hollande) sont des choix, ces entités n'ayant pas de capitale fixe.
- **Étain** de Cornouailles et du Devon représenté par `res_iron` faute de ressource dédiée ; **argent** de Freiberg, Kutná Hora, Iglesias idem.
- **Normandie** et **Bretagne** sont désormais chacune en deux provinces ; les populations de l'échantillon M0 ont été réparties en conséquence.
- Noms locaux en langues d'époque : orthographes reconstituées (ancien français, moyen anglais, scots, occitan, gascon...), à considérer comme indicatives.

## 5. Sources (titres de pages Wikipédia)

Chaque fichier porte ses propres `sources`. Pages de synthèse consultées pour les propriétaires de 1337 :
Guerre de Cent Ans ; Duché d'Aquitaine ; Guerre de Saint-Sardos ; Comté de Ponthieu ; Comté de Flandre ; Transport de Flandre ; Comté d'Artois ; Duché de Bourgogne ; Comté de Bourgogne ; Duché de Bretagne ; Guerre de Succession de Bretagne ; Comté de Champagne ; Jeanne II de Navarre ; Comté d'Angoulême ; Dauphiné de Viennois ; Humbert II de Viennois ; Comté de Provence ; Robert Ier de Naples ; Comtat Venaissin ; Papauté d'Avignon ; Seigneurie de Montpellier ; Royaume de Majorque ; Comté de Savoie ; Savoie-Achaïe ; Duché de Bourbon ; Comté d'Armagnac ; Vicomté de Béarn ; Comté de Périgord ; Comté d'Auvergne ; Comté de Bar ; Duché de Lorraine ; Comté de Luxembourg ; Comté de Hainaut ; Comté de Hollande ; Duché de Brabant ; Duché de Gueldre ; Principauté de Liège ; Principauté épiscopale d'Utrecht ; Comté de Namur ; Électorat de Cologne ; Électorat de Trèves ; Électorat de Mayence ; Palatinat du Rhin ; Traité de Pavie (1329) ; Louis IV du Saint-Empire ; Marche de Brandebourg ; Marche de Misnie ; Royaume de Bohême ; Duché d'Autriche ; Comté de Tyrol ; Ancienne Confédération suisse ; Liberté frisonne ; Comté de Holstein ; Interrègne danois (1332-1340) ; Magnus IV de Suède ; Azzone Visconti ; République de Gênes ; Marquisat de Montferrat ; Mastino II della Scala ; République de Venise ; Maison d'Este ; Taddeo Pepoli ; République de Florence ; Marche d'Ancône ; Patrimoine de saint Pierre ; Royaume de Naples ; Royaume de Sicile ; Royaume de Sardaigne ; Histoire de la Corse ; Couronne de Castille ; Alphonse XI de Castille ; Seigneurie de Biscaye ; Royaume de Murcie ; Bataille du Salado ; Royaume de Grenade ; Siège de Gibraltar (1333) ; Pierre IV d'Aragon ; Principauté de Catalogne ; Royaume de Valence ; Royaume de Navarre ; Alphonse IV de Portugal ; Guerre luso-castillane (1336-1339) ; Duché de Cornouailles ; Cinque Ports ; Londres au Moyen Âge ; Comté de Lancastre ; Principauté de Galles ; Marches galloises ; Seconde guerre d'indépendance écossaise ; Édouard Balliol ; Robert II d'Écosse ; Andrew Murray (1298-1338) ; Seigneurie d'Irlande ; Comté d'Ulster ; Comte de Desmond.
