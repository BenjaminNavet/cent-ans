# Audit historique des bâtiments, ressources et repères — 25 septembre 2026 (lot B4)

Relecture des noms et descriptions de `data/buildings/` (30), `data/resources/` (10) et `data/landmarks/` (7),
pour une partie commençant au printemps 1337. Mêmes catégories que l'audit H1 (`audit-2026-09-23.md`) :

- **Erreur** : fait faux, corrigé dans le fichier.
- **Approximation assumée** : discutable, laissée telle quelle et signalée.
- **Anachronisme de jeu** : écart connu, conservé pour la jouabilité.

Seuls les noms, descriptions et sources ont changé ; aucun coût, effet ni prérequis n'a été modifié.
Les descriptions reçoivent en outre un lien `[[cdx_…]]` vers leur fiche du Codex.

## Corrections

### Bâtiments (`data/buildings/`)

| Entité | Champ | Avant | Après | Catégorie | Justification | Source |
|---|---|---|---|---|---|---|
| bld_forge | description | « Forge et haut fourneau primitif » | « Bas fourneau et forge au charbon de bois, parfois à soufflets et martinet hydrauliques » | Erreur | Le haut fourneau n'apparaît qu'au XIVe-XVe s. en Suède et dans le Pays de Liège, en Angleterre en 1496 ; le fer de 1337 se fait au bas fourneau | Bas fourneau ; Haut fourneau |
| bld_hotel_dieu | description | « soins, hospitalité des pauvres, quarantaine sommaire » | « accueil des pauvres, des pèlerins et des malades, soin de l'âme et du corps » | Erreur | La quarantaine naît à Raguse en 1377 ; les hôtels-Dieu, loin d'isoler, refusaient en principe lépreux et contagieux | Hôtel-Dieu ; Quarantaine |
| bld_fair | description | « Foire franche à privilèges royaux (Champagne, Lendit, Bruges) » | « Foire à privilèges (foires de Champagne, déjà sur le déclin en 1337, Lendit, Chalon, Stourbridge) » | Erreur | Les privilèges des foires de Champagne sont d'abord comtaux, et elles déclinent dès la fin du XIIIe s. ; Bruges est un marché permanent plutôt qu'une foire | Foires de Champagne |
| bld_parish_church | description | « encadrement des fidèles, dîme, registres » | « sacrements, prône et dîme ; à Givry, un curé tient dès 1334 l'un des premiers registres de sépultures » | Approximation corrigée | Les registres paroissiaux ne sont obligatoires qu'au XVIe s. (1538-1539) ; celui de Givry est une exception | Registres paroissiaux |
| bld_guild_hall | description | « Siège des corporations » | « Siège des métiers jurés et des guildes » | Précision | « Corporation » est un terme du XVIIIe s. ; on dit alors métiers, guildes, jurandes | Corporation (Ancien Régime) |
| bld_counting_house | description | « Loge des marchands italiens ou hanséates ; lettres de change et crédit » | « Succursale d'une compagnie italienne (Bardi, Peruzzi) ou comptoir de la Hanse (Steelyard) : dépôts, crédit et, chez les Italiens, lettres de change » | Précision | Les Hanséates se méfient de la lettre de change, instrument italien | Lettre de change ; Hanse |
| bld_archery_butts | description | « le dimanche, sur ordre royal en Angleterre » | « les dimanches et jours de fête, sur ordre d'Édouard III (1363) » | Précision | Proclamation d'Édouard III aux shérifs, 1363 | Arc long anglais |
| bld_armoury | description | « (Tour de Londres, Louvre) » | « Privy Wardrobe de la Tour de Londres, artillerie du Louvre, Clos des galées de Rouen » | Précision | Le Clos des galées fabrique armes et premières armes à feu (1338) | Clos des galées de Rouen |
| bld_castle | description | « enceinte concentrique » | « enceinte flanquée de tours, parfois concentrique » | Précision | L'enceinte concentrique (châteaux gallois d'Édouard Ier) reste rare en France | Château fort |
| bld_university | description | « à privilèges pontificaux » | « aux privilèges pontificaux ou royaux » | Précision | Oxford est un studium generale ex consuetudine, sans bulle de fondation | Université médiévale |
| bld_muster_field | description | « où l'on passe les montres des troupes levées » | « où les capitaines présentent leurs troupes à la montre, pour être comptées et payées » | Précision | La montre conditionne la solde (ordonnance de 1351) | Ost (armée) |
| bld_water_supply | description | « Fontaines publiques, conduites et égouts » | « Fontaines publiques, aqueducs, conduites et premiers égouts voûtés (Paris, Londres) » | Précision | Égout voûté d'Hugues Aubriot (v. 1370), Great Conduit de Londres (1245) | Hugues Aubriot ; Great Conduit |
| bld_tin_blowing_house | description | « monnayés, c'est-à-dire pesés et taxés » | « essayés, pesés et taxés » | Précision | Le coinage consiste d'abord à couper un coin du lingot pour l'essayer | Stannaries |
| 26 bâtiments | sources | « Économie médiévale », « Architecture militaire médiévale » pour presque tous | Articles propres à chaque bâtiment | Nettoyage | Sources génériques sans rapport (architecture militaire pour un marché ou un hôpital) | — |

### Ressources (`data/resources/`)

| Entité | Champ | Avant | Après | Catégorie | Justification | Source |
|---|---|---|---|---|---|---|
| res_wine | description | « Vins de Gascogne (Bordeaux), de Bourgogne et d'Île-de-France, exportés en Angleterre » | « Vins de Gascogne (Bordeaux) et de La Rochelle, exportés vers l'Angleterre et la Flandre ; vins de Bourgogne et d'Île-de-France pour Paris et la cour d'Avignon » | Erreur | Le vin exporté en Angleterre est gascon et rochelais ; Bourgogne et Île-de-France alimentent Paris, la Flandre et Avignon | Vignoble de Bordeaux ; Vignoble de Bourgogne |
| res_fish | description | « Hareng de la mer du Nord et de la Manche, morue » | « Hareng de Scanie, de la mer du Nord et de la Manche, morue séchée de Norvège (stockfisch), poisson des étangs » | Précision | Au XIVe s., les grands bancs de hareng sont en Scanie ; la morue consommée est le stockfisch norvégien | Pêche au hareng ; Stockfisch |
| res_wool | description | « Laine anglaise et castillane » | « Laine anglaise, la plus recherchée, puis castillane (mérinos de la Mesta, surtout après 1400) » | Précision | Les exportations de laine castillane vers la Flandre ne décollent qu'à la fin du XIVe s. | Mesta |
| res_iron | description | « Weald du Kent, Berry, Lorraine » | « Weald du Sussex et du Kent, forêt de Dean, Berry, Lorraine, Biscaye, Pays de Liège » | Précision | Le fer du Weald est surtout sussexois ; forêt de Dean et Biscaye sont des centres majeurs | Sidérurgie |
| res_cloth | description | « premier produit d'exportation » | « premier produit manufacturé du commerce européen » ; ajout de l'Angleterre | Précision | Formulation ambiguë ; l'Angleterre devient exportatrice de draps après 1350 | Draperie |
| res_wheat | description | « Céréale de base » | « Froment et autres blés (seigle, méteil, orge) » | Précision | Les « blés » médiévaux désignent toutes les céréales panifiables | Blé ; Méteil |
| res_salt | description | Guérande, Saintonge, Bourgneuf | + salines de Salins et de Lorraine ; « gabelle en France » | Précision | Sel ignigène de l'intérieur ; la gabelle est un impôt français | Marais salant |
| res_stone, res_wood | description | — | Précisions (tuffeau, pierre de Caen ; if importé, charbon des forges) | Précision | — | Pierre de Caen ; If commun |

### Codex (fiches existantes touchées par B4)

| Fiche | Avant | Après | Catégorie |
|---|---|---|---|
| cdx_tin_stannaries | « le contrôle de la Couronne sur le contrôle et la frappe (« coinage ») de l'étain » | « sur l'essai et le poinçonnage (« coinage ») » | Erreur (le coinage n'est pas une frappe de monnaie) |
| cdx_places_fortes | alias « château fort », « châteaux forts » | déplacés vers la nouvelle fiche `cdx_chateau_fort` | Réorganisation |
| cdx_hotel_dieu | `entity` tech_hospital_reform | `entity` bld_hotel_dieu | Réorganisation (la technologie perd son lien de titre ; à reprendre par B5 si besoin) |

## Approximations assumées et anachronismes de jeu (non corrigés)

- **Moulin à eau amélioration du moulin à vent** (`bld_water_mill.upgrades_from = bld_windmill`) : ordre historique inverse (moulin à eau antique, moulin à vent européen vers 1180). Signalé dans la fiche `cdx_moulin_a_eau` (champ `anachronism`).
- **Abbaye amélioration de la collégiale** : deux institutions distinctes (chanoines séculiers / moines réguliers). Signalé dans `cdx_abbaye`.
- **Boulevard d'artillerie remplaçant le château fort** : avec les effets actuels, il fait perdre 2 niveaux de fortification, la garnison +3 et la loyauté +5 du château. Problème d'équilibrage plus qu'historique ; à arbitrer (effets cumulés ou valeurs relevées).
- **Nom « Arsenal »** : le mot (venu de l'Arsenal de Venise) n'entre dans le français courant qu'à la fin du Moyen Âge ; conservé comme nom de jeu.
- **Pierre en Normandie** : la province de Normandie ne produit pas `res_stone`, alors que la pierre de Caen est la plus exportée du temps. Donnée de province, hors périmètre B4.
- **Gabelle** : la fiche `cdx_gabelle` date l'institution de 1341, l'événement `evt_gabelle_du_sel` cite une ordonnance du 20 mars 1342 (a. st.). Cohérence à vérifier par B6.
