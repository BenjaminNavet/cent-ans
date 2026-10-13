# Paysages agraires vers 1340 (GR lot 3)

Audit de `data/map/agri_landscapes.json` et de `data/art/ground_biome_mix.json` (ADR 0352). Sources de référence : Marc Bloch, *Les caractères originaux de l'histoire rurale française* (1931) ; Roger Dion, *Histoire de la vigne et du vin en France des origines au XIXe siècle* (1959) ; Georges Duby, *L'économie rurale et la vie des campagnes dans l'Occident médiéval* (1962) ; H. E. Hallam (éd.), *The Agrarian History of England and Wales II, 1042-1350* (1988) ; B. M. S. Campbell, *English Seigniorial Agriculture 1250-1450* (2000) ; Oliver Rackham, *The History of the Countryside* (1986) ; Thirsk (éd.), *Agrarian History of England and Wales IV* (pour les systèmes régionaux) ; Thoen et Verhulst pour la Flandre ; Glick, *Islamic and Christian Spain* et Guinot sur la huerta valencienne ; Jones/Pinto sur la Toscane. Les références de pages ne sont pas citées de mémoire : seuls auteur, titre et chapitre thématique sont donnés. Les incertitudes sont marquées « (?) ».

## 1. Anachronismes de cultures retirés

| Culture | Où | Décision | Raison |
|---|---|---|---|
| Sarrasin (`buckwheat_field`) | biomes 1, 2, 5, 11, 13 ; paysage `bocage_west` | retiré (remplacé par seigle ou blé) | Introduit en Europe occidentale au XVe s. (Bretagne : premières mentions fin XIVe-XVe s. ; l'essor est du XVIe s.). Dion/Bloch : absent des assolements de 1340. |
| Sarrasin | biome 10 (forêt-steppe) | conservé | Rus' kiévienne : *grétchikha* attestée depuis les Xe-XIe s. (origine byzantine). |
| Sarrasin | `swidden_east` | 1,5 → 0,8 | Seule la Moscovie/Novgorod/Lituanie est fondée ; Finlande et Suède (Småland) ne le cultivent pas avant le XVe-XVIe s. (?) : poids réduit plutôt que retiré, le paysage étant partagé. |
| Riz (`rice_paddy`) | Lombardie, Piémont | région basculée sur `openfield_north` | Riziculture padane introduite vers 1468-1475 (Sforza, Lomellina) ; vers 1340 la plaine irriguée est faite de blé et de marcite (prés irrigués, cisterciens depuis le XIIe s.). Source : Duby, ch. sur l'Italie ; Cipolla (?). La marcite n'a pas de paysage dédié (voir § 4). |
| Riz | delta de l'Èbre | région `ebre` supprimée | Riziculture du delta : XIXe s. La Ribera d'Èbre garde les oliveraies de `catalunya`. |
| Riz | Valence, Murcie, Grenade | conservé | Riz de l'Albufera attesté depuis l'époque andalouse, encadré par le Fuero de Valence (1250) ; Murcie (Segura) documenté. Grenade : (?) peu de riz, surtout canne, mûrier, huerta : tolérance, couleur identique à la huerta. |

Pas de maïs, pomme de terre, tomate, tabac ni agrume dans les données : rien d'autre à retirer. Le cidre (`apple_orchard`) est normand dès le XIIe s. ; en Bretagne il est plus tardif (?) : poids du verger ramené de 1,5 à 1,2 dans `bocage_west`.

## 2. Nouvelles cultures

- `chestnut_grove` (châtaigneraie) : couleur d'été `[0.06, 0.115, 0.035]`, classe 6 (pérenne), repli de matière `apple_orchard` (`material_fallbacks`). Pas de nouvelle texture.

## 3. Paysages ajoutés (2 sur 2 permis, lignes 14 et 15 de la table)

- **`upland_pastoral`** (pâtures d'altitude et landes d'élevage) : pâture 3,5, foin 1,5, orge 0,6 (*bere*), jachère 0,7, labour 0,3 ; `coteau_vine` 0. Justification : Galles (économie pastorale, transhumance *hafod/hendre*, culture limitée aux fonds : Hallam II, chap. sur Galles et Marches ; Davies, *Conquest, Coexistence and Change: Wales 1063-1415*), Écosse méridionale et Highlands (élevage ovin des abbayes de la frontière, Melrose et Kelso, grands exportateurs de laine : Duncan, *Scotland: the Making of the Kingdom*), Pennines et Cumbrie (granges cisterciennes à moutons : Hallam II), Irlande gaélique (économie bovine : Duffy, *Ireland in the Middle Ages*), Haute-Auvergne/Aubrac (estives : Duby).
- **`chestnut_upland`** (châtaigneraies et petites terrasses) : châtaigneraie 4,0, seigle 2,0, pâture 2,0, foin 1,0, jachère 1,0, vigne 0,3 ; `coteau_vine` 0,05. Justification : le châtaignier « arbre à pain » de Limousin, Périgord noir, Cévennes, Vivarais s'étend aux XIe-XIIIe s. avec les défrichements monastiques et seigneuriaux (Bruneton-Governatori, *Le pain de bois*, 1984 ; Duby ; Bloch sur les terroirs de l'ouest du Massif central). Apennin ligure et Bierzo : même système (?).

## 4. Régions ajoutées (40) et modifiées

Légende : OF = `openfield_north`, UP = `upland_pastoral`, CH = `chestnut_upland`, BO = `bocage_west`, VT = `vineyard_terraces`.

### Champs ouverts (OF)
- `plaine_de_france` (Plaine de France, Hurepoix), `plaine_caen`, `caux` : plaines céréalières à champs ouverts, grenier de Paris et de Rouen ; pays de Caux : champs ouverts avec fermes closes (*clos-masures*) (Bloch ch. 2 ; Duby). Plaine de Caen : (?) l'ellipse du Bocage normand la recouvrait.
- `lorraine_plateau` : champs longs et soles contraintes, Bloch (ch. 2, « les champs allongés de Lorraine »).
- `lauragais` : grenier à blé du Toulousain, pastel (Wolff, *Commerces et marchands de Toulouse*). Le paysage « openfield du nord » est une approximation de la structure parcellaire (?).
- `kent_est`, `wessex`, `oxfordshire`, `lincolnshire`, `vale_of_york` : système champenois (« Midland system ») ou sheep-corn des downs, champs ouverts à lanières (Hallam II ; Campbell ; Thirsk IV).
- `pale_irlandais` : seigneuries anglo-normandes du Leinster à manoirs et champs ouverts (Duffy). Étendue (?).
- `tierra_de_campos` : céréaliculture de Vieille-Castille, assolement biennal (Glick ; Duby). Structure parcellaire (?).
- `lombardia`, `piemont_riz` : passées de `irrigated_huerta` à OF, voir § 1.
- `flandre_hainaut` : renommée « Artois, Hainaut et Flandre intérieure », recentrée à 50,5° N ; Thoen/Verhulst : intensification flamande (jachère réduite, lin, navets) dès la fin XIIIe s. mais en structure d'openfield pour l'intérieur.

### Élevage (UP)
`pays_de_galles`, `ecosse_sud`, `pennines`, `highlands`, `irlande`, `haute_auvergne` : voir § 3. Les emprises sont larges (ellipses à 100 km) ; la plaine côtière de Glamorgan et le Lothian, céréaliers, ne sont pas corrigés (?).

### Châtaigneraies (CH)
`limousin`, `perigord`, `cevennes`, `vivarais`, `apennin_ligure`, `bierzo` : voir § 3. Le Périgord noir est mixte (châtaigne, noyer, vigne) (?).

### Bocage (BO)
- `flandre_maritime` : polders, fossés et prés d'embouche de la plaine maritime (Verhulst, *The Rise of Cities in North-West Europe*; Thoen). Le bocage_west est une approximation (haies = fossés).
- `minho` : petites parcelles closes d'entre Douro et Minho, vigne en treille haute sur arbres (*vinho verde*) (?).
- Bocage de l'ouest (existant) : la date de la fermeture des parcelles est débattue ; Bloch la tient pour ancienne en Bretagne, mais l'extension du bocage normand et manceau est surtout postérieure à la guerre de Cent Ans (Antoine, *Le bocage*). Non modifié, incertain.

### Vignobles (VT) ajoutés
Dion, partie « Le Moyen Âge » ; limite septentrionale de la vigne vers 1340 : Basse-Normandie (en recul), vallées de la Seine et de l'Oise, Picardie (Laon, Soissons), Champagne, Meuse et Moselle, Rhin moyen et Saale-Unstrut, avec l'Angleterre et la Flandre en dehors de l'aire utile.
- `ile_de_france_vigne` (Montmartre, Argenteuil, Suresnes, Meudon, Bagneux ; Paris et son vignoble, Dion), `vexin_vigne` (Mantois, Vexin, Gaillon, Les Andelys ; vignes normandes, Dion), `auxerrois` (Auxerre, Chablis, Joigny : vin du Paris médiéval par l'Yonne), `orleanais_vigne` (Orléans, Beaugency, Blois : exportateurs de vin vers Paris, Dion), `maconnais`, `jura_vigne` (Arbois), `savoie_vigne` (Chignin, Apremont ; (?)), `lorraine_vigne` (Côtes de Toul, Pont-à-Mousson, Metz).
- `saintonge_aunis` : vignoble d'exportation de La Rochelle vers l'Angleterre, la Flandre et la Hanse (Dion ; Renouard, *Études d'histoire médiévale*).
- `cahors`, `gaillac`, `bergerac` : vins du Haut-Pays descendus par la Garonne et la Dordogne vers Bordeaux (Dion ; Higounet, *Histoire de l'Aquitaine*).
- `rioja` : vigne de la vallée de l'Èbre depuis le XIe s. (Pèlerin de Compostelle, œnologie monastique : (?) pour l'ampleur), `ribeiro` : vins de Galice exportés vers l'Angleterre dès le XIVe s. (?).

### Régions existantes modifiées
- `bordelais` : recentré à (-0,35 ; 44,85), rayons 50×25 : le Médoc n'est drainé et planté qu'à l'époque moderne ; le vignoble de 1340 est la Graves, l'Entre-deux-Mers et le Libournais (Dion).
- `ebre` : supprimée. `alentejo` et `sicilia` : conservées malgré des doutes (§ 6).
- `douro` (Haut-Douro) : conservé mais la fonction viticole d'exportation est du XVIIIe s. ; la vigne y existe dès le XIIe s. (cisterciens de São João de Tarouca) (?).
- `tokaj` : la viticulture se développe après 1241 avec des colons wallons et italiens ; la renommée est moderne (?).

## 5. Part de vigne sur coteaux (`coteau_vine`, Dion)

| Cible | Avant | Après | Raison |
|---|---|---|---|
| `openfield_north` | 0,30 | 0,08 | Le paysage couvre la Flandre, les Midlands, East Anglia et la Cujavie, hors aire de la vigne ; les terroirs viticoles du nord ont désormais leurs régions VT. |
| biome 1 océanique | 0,15 | 0,06 | Angleterre : vignes seigneuriales et monastiques en recul depuis 1300 (Hallam II ; Rackham). |
| biome 2 continental | 0,35 | 0,30 | Couvre aussi la Pologne et la Bohême ; le Bassin parisien, la Rhénanie sont couverts par VT. |
| `chestnut_upland` | – | 0,05 | Vigne de coteau sporadique (Cévennes). |
| `upland_pastoral` | – | 0,0 | Hors de l'aire de la vigne. |
| autres | inchangé | | `vineyard_terraces` 0,6 ; `olive_orchard_south` 0,35 ; biomes 3 (0,45), 7, 11 (0,35), 12, 14 (0,4) conformes à Dion. |

## 6. Incertitudes ouvertes

- Rizières : poids du riz de la huerta (Murcie, Grenade) à vérifier ; la marcite lombarde n'a pas de paysage propre (un 9e paysage dépasserait la limite de 8 lignes 8..15).
- `sicilia` et `alentejo` rendues en `olive_orchard_south` (oliveraie dominante) ; vers 1340 ce sont d'abord des pays céréaliers (blé sicilien, latifundia ; Alentejo céréalier : Duby). À repasser en biome semi-aride si la carte le permet.
- Bocage (Normandie, Maine, Vendée) : date de la fermeture des parcelles disputée.
- Emprises d'ellipses approximatives : à contrôler visuellement (planche `game/tests/gr_regions_shot.gd`, non lancée ici, pas de capture dans ce lot).
- Pas de région pour les Landes de Gascogne (landes à moutons et pins du XIXe s. : biome 11), la Brie champenoise ni le Poitou plaine (champs ouverts) : laissés au biome.
