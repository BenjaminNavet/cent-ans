# VH — Dossier de sources pour les villes historiques et le peuplement

Dossier de sources pour le chantier VH (`docs/archive/chantiers.md`), demandé le
2026-09-25, avant même que ZG2/ZG4 ne fixent l'API du relief (VH0). Ce dossier ne modifie aucun
code ni aucune donnée de jeu : il sert de référence pour choisir les sources ouvertes de VH1
(forêts), VH2 (villages), VH4 (parcellaire générique), VH5 (Paris), VH6 (Londres), VH7 (Orléans) et
VH8 (les six autres villes emblématiques).

Méthode : chaque source a été vérifiée par recherche web sur sa page officielle (licence, emprise,
résolution, format, URL). Aucun gros jeu de données n'a été téléchargé — seules les pages de
description et de licence ont été consultées. Aucun fait historique n'a été inventé : ce qui n'est
confirmé que par une seule source, ou par des sources qui se contredisent, est marqué
« incertain » et documenté comme tel plutôt que résolu arbitrairement.

Ce dossier reprend et vérifie certaines sources déjà citées dans le projet
(`data/landmarks/paris.json`, `data/map/historical_forests.json`, ADR 0015) ; quand la vérification
confirme un usage déjà en place, le dossier le signale sans le remettre en cause. Quand elle
identifie un problème (licence non commerciale, chiffre daté d'une autre époque que celle utilisée
dans le JSON), le dossier le signale explicitement dans les notes et dans les recommandations
(section 4).

## Sommaire

1. Sources de données ouvertes (forêts, villages, villes)
2. Faits datés par ville : Paris, Londres, Orléans
3. Parcellaire urbain médiéval typique
4. Recommandations par lot (VH1, VH2, VH4-VH7)

---

## 1. Sources de données ouvertes

### 1.1 Forêts (VH1)

| Nom | Producteur | Emprise | Résolution / échelle | Format | Licence exacte (vérifiée) | URL de téléchargement / API | Compatibilité jeu distribué | Usage VH1 |
|---|---|---|---|---|---|---|---|---|
| ESA WorldCover 2021 (v200) | ESA / VITO (consortium WorldCover) | Mondiale (tuiles 1°×1°) | ~10 m (composite SWIR à 20 m) | Cloud-Optimized GeoTIFF (COG), WGS84 | **CC BY 4.0** — vérifiée sur esa-worldcover.org/en/data-access : « fourni gratuitement, sans restriction d'utilisation », attribution requise | AWS S3 `s3://esa-worldcover/v200/2021/map` (accès public sans clé) ; Zenodo https://doi.org/10.5281/zenodo.7254221 | Oui — attribution simple, pas de clause NC ni de partage à l'identique | Couche de base « couvert arboré actuel » (déjà utilisée par ZG1) que VH1 corrige avec les couches historiques ci-dessous |
| BD FORÊTS ANCIENNES | IGN | France métropolitaine | Croisement carte d'État-Major (1820-1866) × BD Forêt v2.0 (2007-2018), seuil 500 m² | WMTS/WMS, vecteur via API OGC (cartes.gouv.fr) | **Licence Ouverte / Open Licence v2.0 (Etalab)** — vérifiée sur data.gouv.fr/datasets/bd-forets-anciennes : réutilisation libre y compris commerciale, simple mention de la source | https://cartes.gouv.fr/catalogue/dataset/IGNF_BD-FORETS-ANCIENNES ; https://www.data.gouv.fr/datasets/bd-forets-anciennes | Oui, y compris usage commercial, attribution simple | **Source principale recommandée** pour distinguer forêt ancienne (probablement déjà boisée en 1340) de forêt récente ou disparue ; corrige WorldCover |
| Carte de Cassini vectorisée — forêts (projet Cartofora, WWF France / INRA) | GIP Ecofor (Cartofora), vectorisation Vallauri et al. | France métropolitaine, 181 planches Cassini au 1:86 400 | 1:86 400 | Rapport + CD-Rom SIG (Vallauri et al. 2012) ; aucun portail de téléchargement actif retrouvé (page Cartofora inaccessible au moment du contrôle) | **Non vérifiable en ligne** : aucune page de licence trouvée pour les polygones vectorisés eux-mêmes | Rapport PDF : http://docs.gip-ecofor.org/public/forets_anciennes/Vallauri_et_al_Cassini_2012.pdf (pas les données) | **À écarter en l'état** : ni portail actif, ni licence vérifiée | Écarté pour VH1 ; remplacé fonctionnellement par BD FORÊTS ANCIENNES (IGN), qui couvre le même besoin avec une licence claire |
| Ancient Woodland Inventory (England) | Natural England | Angleterre (hors îles Scilly), ~365 000 ha, ~53 600 polygones | Vectoriel, seuil historique ~2 ha | Shapefile, GeoPackage, GeoJSON, Geodatabase Esri ; OGC API Features, WFS/WMS | **Open Government Licence v3.0** — vérifiée sur data.gov.uk : réutilisation libre y compris commerciale, attribution + mention Crown copyright | https://www.data.gov.uk/dataset/9461f463-c363-4309-ae77-fdcd7e9df7d3/ancient-woodland-england ; https://naturalengland-defra.opendata.arcgis.com/datasets/Defra::ancient-woodland-england/about | Oui, y compris commercial, attribution Crown copyright | Équivalent anglais de BD FORÊTS ANCIENNES pour les provinces anglaises (Guyenne, possessions Plantagenêt) |
| Royal Forests médiévales (limites juridiques des forêts royales anglaises, XIIIe-XIVe s.) | Aucun producteur SIG identifié | — | — | — | Sans objet — aucune couche SIG ouverte trouvée (recherché via data.gov.uk, Historic England Open Data Hub, Forest Research Open Data) | — | Sans objet | **Recommandation : ne pas chercher de couche dédiée.** S'en tenir aux ellipses déjà présentes dans `historical_forests.json` (Sherwood, New Forest, Forest of Dean, Windsor, Epping/Waltham…), sourcées via Bazeley 1921 |
| KK10 — Kaplan & Krumhardt 2011 (Anthropogenic Land Cover Change) | J. O. Kaplan, K. M. Krumhardt | Mondiale, non projetée | Grille 5' d'arc (4320×2160), pas annuel, 8000 BP-1850 AD | NetCDF v4 (~17,3 Go pour la série complète) | **CC BY 3.0** — vérifiée sur PANGAEA https://doi.pangaea.de/10.1594/PANGAEA.871369 | https://hs.pangaea.de/model/ALCC/KK10.nc (fichier volumineux — noter l'URL, ne pas télécharger) | Oui, attribution simple | Déjà utilisé (cité par `historical_forests.json`) ; pour VH1, extraire seulement les tranches temporelles proches de 1340 |

Notes :

- La vectorisation Cassini (Cartofora/WWF-INRA) est différente de BD FORÊTS ANCIENNES (IGN) : la
  première repose sur la carte de Cassini (1750-1789), la seconde sur la carte d'État-Major
  (1820-1866) croisée avec BD Forêt (2007-2018), diffusée avec une licence claire et un
  téléchargement direct. Pour VH1, retenir uniquement BD FORÊTS ANCIENNES.
- Aucune des sources retenues pour VH1 n'impose de clause non commerciale ni de partage à
  l'identique de type ODbL ; toutes n'exigent qu'une attribution simple, à consigner dans un
  fichier de crédits du jeu (à créer si absent, hors périmètre de ce dossier).
- Aucun problème de licence bloquant pour VH1.

### 1.2 Villages (VH2)

| Nom | Producteur | Emprise | Résolution / échelle | Format | Licence exacte + URL de vérification | URL de téléchargement / API | Compatibilité jeu distribué | Usage VH2 |
|---|---|---|---|---|---|---|---|---|
| Des villages de Cassini aux communes d'aujourd'hui (base Cassini-Geopeuple) | EHESS (LADHS) / BnF / CNRS / INED | France entière, ~37 000 lieux figurant sur la carte de Cassini (1756-1789) | Points géoréférencés par commune/lieu | Consultation web fiche par fiche (cassini.ehess.fr) ; une version SIG (« Geopeuple ») existe, non vérifiée en détail (page inaccessible depuis l'environnement de recherche) | **CC BY-NC-SA 3.0 France** — trouvé via une page tierce (mentions légales EHESS), **la page de licence officielle elle-même n'a pas pu être chargée** : à reconfirmer directement sur cassini.ehess.fr avant usage | http://cassini.ehess.fr/ ; http://sig.cassini-geopeuple.huma-num.fr/ (à vérifier) | **Bloquant si confirmé** : la clause NC interdit tout usage dans un jeu distribué commercialement | Prévu pour les positions de villages/hameaux français ; **à traiter comme source de contrôle non redistribuable** en l'état (voir recommandations §4) |
| Open Domesday — données brutes (Domesday Book, 1086) | A. Powell-Smith (site) ; données hébergées par l'université de Hull (J. Palmer, G. Slater) | Angleterre (comtés couverts par le Domesday de 1086) | Points par manoir, valeurs tabulaires (feux/hides, valeur, population estimée) | Site interrogeable ; API mentionnée ; données brutes hébergées séparément (hydra.hull.ac.uk) | Le site distingue deux régimes, confirmés sur opendomesday.org/about/ : **données brutes en CC-NC-BY-SA** ; **images de folios en CC BY-SA 3.0** avec attribution. **Le plan VH suppose CC BY-SA pour l'ensemble — c'est inexact : seules les images sont réutilisables commercialement, les positions et valeurs sont NC** | https://opendomesday.org/about/ ; https://opendomesday.org/api/ ; https://hydra.hull.ac.uk/resources/hull:domesdayDisplaySet | **Bloquant pour les données tabulaires** (clause NC) ; les images de folios seules seraient compatibles mais n'apportent pas les positions/valeurs utiles | Prévu pour les villages anglais (Domesday, ≈250 ans avant 1340). **Correction nécessaire au plan** : soit obtenir une autorisation explicite auprès de Hull/Powell-Smith, soit reconstituer indépendamment les toponymes/positions à partir du texte du Domesday Book (XIe siècle, domaine public), sans réutiliser la base structurée de Hull |
| L'état des paroisses et des feux de 1328 (édition Ferdinand Lot, 1929) | F. Lot, *Bibliothèque de l'École des chartes*, t. 90, 1929 | France (bailliages et sénéchaussées du domaine royal uniquement) | Dénombrement textuel par paroisse et bailliage (23 671 paroisses, 2 469 987 feux) ; pas de coordonnées | Texte scanné/OCR sur Persée ; **aucun jeu tabulaire structuré (CSV/SHP) trouvé** malgré la recherche | Texte de 1929 dans le **domaine public** (auteur mort en 1952) ; les pages Persée sont en accès libre à la lecture ; les valeurs numériques (faits historiques) ne sont pas protégeables, mais la reproduction des scans/mise en forme du portail n'a pas été vérifiée | https://www.persee.fr/doc/bec_0373-6237_1929_num_90_1_448861 (et suite https://www.persee.fr/doc/bec_0373-6237_1929_num_90_1_448863) | Compatible pour les valeurs retranscrites à la main (faits non protégeables) ; incompatible pour une reproduction directe des scans sans vérification des CGU Persée | Prévu pour caler la taille des villages français par bailliage. **Important** : ce recensement **exclut explicitement** la Bretagne, la Bourgogne, la Flandre, la Gascogne anglaise, le Barrois, le Béarn, le Bourbonnais, le Forez, la Marche et plusieurs apanages (Artois, Alençon, Chartres, Évreux, Mortain, Angoulême) — il faut une source de repli (densités par analogie) pour ces provinces |
| Historical Atlas of the Low Countries — GIS Dataset of Locality-Level Boundaries (1350-1800) | R. J. Stapel, KNAW / Huygens Institute, via Dataverse KNAW-HuC | Pays-Bas, Belgique (dont Flandre et Brabant), Luxembourg, parties adjacentes de France/Allemagne | Polygones de localités (paroisses/seigneuries), coupes chronologiques 1350/1500/1650/1800 | Geopackage/shapefile (à confirmer au téléchargement) via Dataverse | **CC BY 4.0**, confirmé par l'article fondateur (R. J. Stapel, *Research Data Journal for the Humanities and Social Sciences*, 8(1), 2023, https://researchdatajournal.org/article/view/23075) | https://hdl.handle.net/10622/PGFYTM (Dataverse KNAW-HuC) | **Compatible**, y compris commercial, sous attribution — pas de clause NC ni de partage à l'identique | **Meilleure source pour la Flandre et le Brabant.** Point de vigilance : à la publication (2023), seule la coupe 1500 était effectivement disponible ; vérifier au moment de l'implémentation si la coupe 1350 (la plus proche de 1337+) est désormais publiée, sinon utiliser la coupe 1500 en l'assumant comme approximation |

Notes complémentaires :

- Aucune base ouverte de paroisses médiévales flamandes/brabançonnes n'a été trouvée en dehors du
  jeu Stapel/KNAW ; les pistes belges usuelles (Geopunt, Informatie Vlaanderen, HisGIS) fournissent
  des données administratives modernes ou des cadastres du XIXe siècle, pas de couche médiévale.
- OpenStreetMap (ODbL 1.0, déjà utilisée dans `data/landmarks/paris.json`) reste une source de
  contrôle possible pour les toponymes conservés, pas une source historique en soi.
- Si la clause NC de Cassini-Geopeuple est confirmée bloquante, une alternative praticable est de
  repartir des toponymes et positions de communes actuelles (IGN Admin Express ou OSM, licences
  ouvertes) et de ne consulter Cassini que comme référence de vérification humaine, sans extraction
  ni redistribution automatisée des 37 000 lieux.

Résumé des risques de licence (villages) :

| Source | Risque | Sévérité |
|---|---|---|
| Cassini-Geopeuple (EHESS) | Clause NC confirmée sur une page tierce, pas sur le site source lui-même | Élevé — à reconfirmer avant tout usage automatisé |
| Open Domesday (données brutes) | Clause NC confirmée par le site officiel lui-même | Élevé — bloquant tel quel pour un jeu commercial |
| Open Domesday (images de folios) | CC BY-SA 3.0, compatible avec attribution | Faible |
| État des paroisses et des feux 1328 (Lot 1929) | Texte de 1929 dans le domaine public ; portail Persée non vérifié pour la reproduction des scans, mais les valeurs comme faits ne sont pas protégeables | Faible à modéré selon l'usage |
| Historical Atlas of the Low Countries (Stapel/KNAW) | CC BY 4.0 confirmé ; coupe 1350 peut-être pas encore publiée | Faible sur la licence, modéré sur la disponibilité de la bonne coupe |

### 1.3 Villes (VH4-VH7)

| Nom | Producteur | Emprise | Échelle / résolution | Format | Licence exacte + URL de vérification | URL d'accès | Compatibilité jeu distribué | Usage VH |
|---|---|---|---|---|---|---|---|---|
| ALPAGE — données SIG (parcellaire Vasserot vectorisé, quartiers, îlots, couches historiques) | Alpage (LAMOP / Huma-Num, coord. H. Noizet) | Paris intra-muros et faubourgs proches | Vectoriel, cadastre napoléonien (Atlas Vasserot, 910 feuilles) géoréférencé, plus couches historiques (enceintes, hôtels, paroisses, seigneuries) | Shapefile + métadonnées | **ODbL 1.0** — vérifiée sur https://alpage.huma-num.fr/gis-data/ | https://alpage.huma-num.fr/gis-data/ ; https://alpage.huma-num.fr/vasserot-data-version-1-2010-a-l-bethe/ | Oui, y compris vente : l'ODbL autorise l'usage commercial, exige l'attribution et, en cas de redistribution de la *base* modifiée, le partage à l'identique de cette base — n'affecte pas les assets dérivés (glTF, textures) | **VH4/VH5 — source à privilégier** pour le parcellaire en lanières, plus riche que l'image de contrôle déjà citée dans `paris.json` |
| Image « Plan de Paris vers 1300-1330 » (ALPAGE, rendu carto) | Alpage / C. Bourlet, N. Thomas, C. Roms | Paris intra-muros, ≈1300-1330 | Image raster (carte publiée) | JPEG/SVG (Wikimedia Commons) | **CC BY 2.0 FR** — déjà vérifiée et citée dans `data/landmarks/paris.json` | https://commons.wikimedia.org/wiki/File:Plan_de_Paris_vers_1300-1330_-_ALPAGE.svg | Oui, avec attribution — image de contrôle, pas une source vectorielle réutilisable | VH5 (contrôle visuel, déjà en usage) |
| Plan de Bâle (Truschet et Hoyau, vers 1550) | Bibliothèque de l'université de Bâle ; reproduction Wikimedia Commons | Paris intra-muros et faubourgs, vers 1550 | Gravure sur bois, sans échelle métrique fixe | Image raster haute résolution | **Domaine public**, déjà vérifiée et citée dans `paris.json` | https://commons.wikimedia.org/wiki/File:Map_of_Paris_by_Truschet_and_Hoyau_-_Basel_University_Library.jpg | Oui, sans restriction | VH4/VH5, déjà en usage |
| Plan de Mérian (1615) et plan de Gomboust (1652) | Diverses institutions ; exemplaires sur Gallica et sur Wikimedia Commons | Paris intra-muros, 1615/1652 | Gravure/plan topographique, image raster | Image raster | Dépend de l'exemplaire : privilégier un exemplaire Wikimedia Commons en domaine public simple plutôt qu'un scan Gallica haute résolution (voir ligne Gallica ci-dessous) | Rechercher « Matthäus Merian Paris 1615 » / « Gomboust plan Paris 1652 » sur Wikimedia Commons | Compatible si l'exemplaire retenu est un domaine public simple hors CGU Gallica commerciales | VH5, recoupement du tracé des rues (postérieur à 1340, utile en contrôle) |
| Fonds numérisés Gallica (BnF) | Bibliothèque nationale de France | France entière | Variable (scans haute résolution) | Image raster (JPEG/TIFF, IIIF) | CGU vérifiées via recherche indexée (accès direct HTTP 403 au moment du contrôle) : **réutilisation non commerciale gratuite avec attribution obligatoire** (« Source gallica.bnf.fr / Bibliothèque nationale de France ») ; **réutilisation commerciale payante et soumise à une licence spécifique BnF**, même pour une œuvre du domaine public | https://gallica.bnf.fr | **Point de licence bloquant pour un jeu vendu** utilisant un export Gallica haute résolution comme asset, sans démarche de licence commerciale. Solution : chercher systématiquement un doublon en domaine public simple sur Wikimedia Commons avant tout export direct | VH5 (plans secondaires), VH7 (plans d'Orléans ci-dessous) — à traiter comme source de *faits*, jamais d'assets copiés |
| Bibliothèque historique de la Ville de Paris (BHVP) | Ville de Paris / BHVP | Paris | Variable (estampes, plans) | Image raster | **Incertain** : une licence CC BY-SA 4.0 est évoquée pour les contenus numérisés de la Ville de Paris, non confirmée sur une page dédiée à un plan précis ; voir aussi le régime général des Archives de Paris (archives.paris.fr/archives-numerisees/reutilisation-des-informations-publiques) | https://www.paris.fr/lieux/bibliotheque-historique-de-la-ville-de-paris-bhvp-16 | Probablement compatible (attribution + partage à l'identique de l'image), à confirmer document par document | VH5, source secondaire non indispensable |
| Map of Early Modern London (MoEML), carte d'Agas | Université de Victoria (J. Jenstad, K. McLean-Fiander et al.) ; original détenu par la City of London/London Metropolitan Archives | Londres intra-muros et Southwark, état vers 1561 (reconstitution) | Plan à vol d'oiseau sans échelle fixe | Image raster (tuiles web) + encodage XML/TEI | **CC BY-NC-SA 4.0** pour l'édition MoEML — vérifiée sur https://mapoflondon.uvic.ca/copyright.htm ; l'image source (London Metropolitan Archives) a une reproduction ultérieure explicitement interdite (« in any form under any circumstances ») | https://mapoflondon.uvic.ca/map.htm ; https://mapoflondon.uvic.ca/agas.htm | **Bloquant pour un jeu vendu** : clause NC + interdiction de reproduction de l'image source, sans accord écrit (contact london@uvic.ca) | VH6 — à utiliser **seulement** comme référence de faits (toponymie, tracé des rues), jamais comme texture/asset copié |
| Historic Towns Trust — British Historic Towns Atlas, « Medieval London » / « Tudor London » (Atlas Vol. III) | Historic Towns Trust | Londres, 1:2 500, vers 1270-1300 et Tudor | Carte imprimée/PDF | PDF | Conditions de réutilisation non consultées en détail (page « How to Use Trust Content » non vérifiée) ; probablement usage académique/attribution stricte | https://www.historictownstrust.uk/maps/medieval-london ; https://www.historictownstrust.uk/atlas/volume-iii | **Incertain** — utilisable sans risque comme source de faits (citation bibliographique), pas comme asset copié, tant que non clarifié | VH6, faits datés |
| Historic England — National Heritage List for England (NHLE) | Historic England | Angleterre entière | Vecteur (points/polygones), mis à jour quotidiennement | CSV, KML, Shapefile, GeoJSON, GeoTIFF ; API GeoServices/WMS/WFS | **Open Government Licence v3** — vérifiée sur historicengland.org.uk/terms/website-terms-conditions/open-data-hub/ | https://opendata-historicengland.hub.arcgis.com/ ; https://historicengland.org.uk/listing/the-list/data-downloads/ | Oui, pleinement compatible, y compris vente, attribution simple | **Source anglaise la plus sûre** pour recaler la Tour de Londres, Old St Paul's, Westminster Hall, Guildhall sur leurs emprises réelles |
| MOLA (Museum of London Archaeology) | MOLA | Londres et fouilles associées | Variable | Variable | **Aucun portail de données ouvertes dédié identifié** ; rapports parfois déposés à l'Archaeology Data Service, conditions par dépôt | https://www.mola.org.uk/research-community/resource-library | Non applicable en l'état — ne pas retenir comme source structurée | Références bibliographiques ponctuelles seulement, à vérifier titre par titre |
| Gallica — plans anciens d'Orléans (siège de 1428, enceintes, plan de 1778) | BnF | Orléans intra-muros et faubourgs | Plans gravés, échelles variables | Image raster (visionneuse Gallica) | Même régime que la ligne Gallica générale ci-dessus | https://gallica.bnf.fr/ark:/12148/btv1b8442118t ; https://gallica.bnf.fr/ark:/12148/btv1b84391001 ; https://gallica.bnf.fr/ark:/12148/btv1b84422897 | Compatible en usage non commercial gratuit uniquement ; pour un jeu vendu, chercher un doublon en domaine public simple ailleurs ou entamer une démarche de licence BnF | VH7 — plans de référence pour le siège de 1428-1429 (déjà objet de l'ADR 0026) et le tracé des enceintes |
| Digital Commonwealth — « Plan et profil au naturel de la ville d'Orléans » | Institution américaine (Massachusetts) | Orléans | Plan gravé ancien | Image raster | Non vérifiée en détail ; à contrôler avant usage | https://www.digitalcommonwealth.org/search/commonwealth:x059c990m | Incertain — piste secondaire, non prioritaire | VH7, source secondaire à ne retenir qu'après vérification |

Notes transverses :

- **Gallica/BnF** : traiter systématiquement comme une source de *contrôle et de faits*, jamais
  d'assets copiés tels quels dans le jeu, sauf démarche de licence commerciale explicite auprès de
  la BnF. Le projet privilégie déjà cette approche pour Paris (plan de Bâle et atlas Legrand cités
  via Wikimedia Commons plutôt que via un export Gallica direct) — à généraliser à Orléans.
- **MoEML/Agas** : clause NC confirmée, blocage net pour un usage direct de l'image ; excellente
  source de faits (toponymie, tracé des rues), à citer en bibliographie, jamais en asset.
- **ALPAGE** : distinguer l'image de contrôle déjà citée (CC BY 2.0 FR) du vrai jeu de données SIG
  (ODbL), bien plus riche et sans ambiguïté pour un usage commercial — à privilégier pour VH4/VH5.
- **Historic England (OGL)** est la source anglaise la plus sûre, sans clause commerciale ni
  redevance, contrairement à MoEML et aux scans Gallica.

---

## 2. Faits datés par ville

Convention : « Oui » = l'élément existe en 1340 dans l'état décrit ; « Non » = il n'existe pas
encore (ou plus). « Incertain » = confirmé par une seule source, ou sources contradictoires (les
deux versions sont alors données). Aucun fait n'a été inventé pour combler une lacune.

### 2.1 Paris

#### Enceintes

| Élément | Existe en 1340 ? | Dates | Dimensions | Source | Fiabilité |
|---|---|---|---|---|---|
| Enceinte de Philippe Auguste (rive droite) | Oui, complète | Construite 1190-1209 | Longueur rive droite ≈2850 m (moins ≈100 m au Louvre) ; longueur totale des deux rives ≈5100 m ; 77 tours semi-cylindriques hautes de 15 m, espacées d'≈60 m ; 10-12 portes fortifiées | [Enceinte de Philippe Auguste, Wikipédia](https://fr.wikipedia.org/wiki/Enceinte_de_Philippe_Auguste) | Recoupé |
| Enceinte de Philippe Auguste (rive gauche) | Oui, complète | Achevée vers 1215-1220 | Comprise dans les 5100 m totaux ; portes Saint-Michel, de Buci, des Cordeliers, Saint-Jacques | idem | Recoupé |
| Surface intra-muros (Philippe Auguste) | — | — | ≈273 ha au total (≈253 ha rive droite + ≈154 ha rive gauche — le détail ne s'additionne pas exactement selon les pages) ; population estimée ≈50 000 hab. à la construction | idem | Total recoupé ; détail par rive incertain |
| Enceinte de Charles V (rive droite) | **Non, pas encore construite** | Terre et fossés 1356-1358 (Étienne Marcel) ; maçonnerie sous Charles V 1365-≈1383 ; achèvement complet ≈1420 sous Charles VI | Une fois achevée : 439 ha intra-muros avec la rive gauche conservée (**une seule source, à confirmer** — ordre de grandeur notable, presque le double de l'enceinte de Philippe Auguste) | [Enceinte de Charles V, Wikipédia](https://fr.wikipedia.org/wiki/Enceinte_de_Charles_V) | Chronologie recoupée ; surface incertaine |

Cohérent avec `data/landmarks/paris.json`, qui date déjà l'enceinte de Charles V `from_year: 1358`
et représente les deux enceintes de Philippe Auguste comme actives en 1340.

#### Monuments et bâtiments

| Élément | Existe en 1340 ? | Dates | Dimensions | Source | Fiabilité |
|---|---|---|---|---|---|
| Louvre — donjon de Philippe Auguste (« Grosse Tour ») | Oui, état de Philippe Auguste ; **pas encore transformé par Charles V** (1364-1380) | Construit ≈1190-1200 | Tour circulaire : diamètre ≈15 m, mur ≈4,20 m à la base, hauteur ≈30-31 m ; fossé sec circulaire ≈10 m de large, 6 m de profondeur | [Château du Louvre, Wikipédia](https://fr.wikipedia.org/wiki/Ch%C3%A2teau_du_Louvre) | Recoupé sur diamètre/épaisseur ; hauteur 30 vs 31 m selon la source |
| Sainte-Chapelle | Oui | Consacrée en 1248 | — | Cohérent avec ADR 0015 / `paris.json` | Recoupé, fait bien établi |
| Tour de l'Horloge (Palais de la Cité) | **Non, pas encore** | Construite 1350-1353 (Jean II le Bon) ; horloge installée en 1371 (Henri de Vic, première horloge publique de Paris) | Hauteur ≈47 m | [Tour de l'Horloge, Wikipédia](https://fr.wikipedia.org/wiki/Tour_de_l%27Horloge_du_palais_de_la_Cit%C3%A9) | Recoupé |
| Notre-Dame — corps de l'édifice | Oui, quasiment achevé | Chantier lancé en 1163 (Maurice de Sully) ; chœur/ambulatoires 1163-1182 ; nef 1182-1190 ; façade et portails 1190-1225 ; achèvement final de la façade et des tours **≈1345** | Tours de façade : hauteur ≈69 m | [Cathédrale Notre-Dame de Paris, Wikipédia](https://fr.wikipedia.org/wiki/Cath%C3%A9drale_Notre-Dame_de_Paris) | Recoupé — en 1340, la cathédrale est achevée ou en voie d'achèvement immédiat |
| Notre-Dame — flèche gothique du XIIIe siècle | Oui | Mise en place probablement dans les années 1290 (dendrochronologie) ; reste en place jusqu'à la fin du XVIIIe siècle | — | [Spire of Notre-Dame, Wikipédia (en)](https://en.wikipedia.org/wiki/Spire_of_Notre-Dame_de_Paris) ; [Flèche de Notre-Dame, Wikipédia (fr)](https://fr.wikipedia.org/wiki/Fl%C3%A8che_de_Notre-Dame_de_Paris) | Recoupé FR/EN — la flèche médiévale (distincte de celle de Viollet-le-Duc) doit apparaître sur le modèle en 1340 |
| Grand-Pont / futur Pont Notre-Dame | Un pont existe à cet emplacement, mais **son état exact en 1340 est incertain** | Le Grand-Pont romain détruit en 887 ; le pont documenté (bois, 106 m × 27 m, 60 maisons) n'est construit qu'en **1413** | 106 m × 27 m, 60 maisons — **valeurs du pont de 1413, pas de 1340** | [Pont Notre-Dame, AFGC](https://www.afgc.asso.fr/history-heritage/pont-notre-dame-a-paris/) | **Incertain pour 1340** — la largeur de 26 m déjà dans `paris.json` (`grand_pont.width_m`) est du même ordre que le pont de 1413, sans garantie pour l'ouvrage de 1340 |
| Petit-Pont | Oui | Reconstruit onze fois avant sa reconstruction en pierre en **1186** (Maurice de Sully) | Longueur ≈40 m | [Petit Pont, AFGC](https://www.afgc.asso.fr/history-heritage/petit-pont-a-paris/) | Recoupé sur longueur et date ; état exact en 1340 (reconstructions intermédiaires) non détaillé |
| Pont Saint-Michel | **Non, pas encore construit** | Généralement daté de 1378-1379 | — | [Pont Saint-Michel, Wikipédia (en)](https://en.wikipedia.org/wiki/Pont_Saint-Michel) | Incertain (une source insuffisamment détaillée), mais cohérent avec `from_year: 1378` déjà dans `paris.json` |
| Grand Châtelet | Oui | Reconstruit en pierre par Louis VI le Gros en 1130 (forme antérieure dès le IXe s.) ; détruit seulement en 1802-1810 | — | [France Pittoresque](https://www.france-pittoresque.com/spip.php?article12238=) | Recoupé |
| Petit Châtelet | Oui | Construit vers 1130, à l'extrémité sud du Petit-Pont | — | idem | Recoupé |
| Temple — enclos et donjon | Oui (bâti), mais **occupant changé** | Enclos >6 ha, mur crénelé de 8-10 m, ≈15 tourelles, porte unique avec pont-levis ; donjon (plan carré, 4 étages) daté entre le frère Hubert (mort 1222) et Jean de Tour (mort ≈1310), débat non tranché ; ordre du Temple supprimé en 1312, domaine transféré aux Hospitaliers | Enclos >6 ha ; mur 8-10 m | [Le donjon du Temple de Paris](https://templedeparis.jimdoweb.com/la-tour-du-temple/) ; [Prieuré hospitalier du Temple, Wikipédia](https://fr.wikipedia.org/wiki/Prieur%C3%A9_hospitalier_du_Temple) | Datation du donjon incertaine dans la littérature elle-même ; le JSON nomme le monument « Enclos du Temple » sans préciser l'ordre propriétaire en 1340 (Hospitaliers depuis 1312, pas Templiers) |
| Abbaye de Saint-Germain-des-Prés | Oui, ancienne | Fondation ≈557 ; nom attesté depuis 754 | — | [Abbaye de Saint-Germain-des-Prés, Wikipédia](https://fr.wikipedia.org/wiki/Abbaye_de_Saint-Germain-des-Pr%C3%A9s) | Recoupé |
| Abbaye Sainte-Geneviève | Oui, ancienne | Fondation traditionnelle en 502 (Clovis et Clotilde) | — | [Abbey of Saint Genevieve, Wikipédia (en)](https://en.wikipedia.org/wiki/Abbey_of_Saint_Genevieve) | Recoupé, mais la date relève en partie de la tradition hagiographique |
| Abbaye Saint-Victor | Existence non contestée en 1340 | **Date de fondation non confirmée dans cette recherche** | — | — | Lacune à combler ; généralement associée au début du XIIe siècle (chanoines réguliers de saint Augustin), mais non vérifié ici |
| Halles de Champeaux | Oui | Marché créé vers 1135 (Louis VI le Gros) ; halles couvertes construites par Philippe Auguste en 1183 ; complétées par Saint Louis en 1269 | — | [Halles de Paris, Wikipédia](https://fr.wikipedia.org/wiki/Halles_de_Paris) | Recoupé |

#### Population

| Élément | Chiffre | Source | Fiabilité |
|---|---|---|---|
| Paris, état des paroisses et des feux de 1328 | Vicomté et justices de Paris : 567 paroisses, 116 986 feux ; **la ville de Paris avec Saint-Marcel : 35 paroisses, 61 098 feux** | F. Lot, *Bibliothèque de l'École des chartes*, 1929, [Persée](https://www.persee.fr/doc/bec_0373-6237_1929_num_90_1_448861) | Chiffre fiscal primaire, largement recoupé |
| Population estimée à partir des feux de 1328 | Estimation basse ≈200 000 hab. (3,5 pers./feu) ; estimations plus hautes de 220 000 à 270 000 | [Paris in the Middle Ages, Wikipédia (en)](https://en.wikipedia.org/wiki/Paris_in_the_Middle_Ages), citant F. Lot | Pas de consensus unique sur la conversion feux → population ; donner la fourchette 200 000-270 000 |

Points restés incertains sur Paris (à surveiller avant de figer VH5) : dimensions exactes du
Grand-Pont en 1340 (celles trouvées datent de 1413) ; date de fondation de l'abbaye Saint-Victor ;
surface exacte de l'enceinte de Charles V (439 ha, une seule source).

### 2.2 Londres

#### Mur

| Élément | Existe en 1340 ? | Dates | Dimensions | Source | Fiabilité |
|---|---|---|---|---|---|
| Mur romain puis médiéval | Oui, en usage constant | Construit par les Romains vers 200 apr. J.-C. ; intégré aux défenses médiévales sur le même tracé | Longueur ≈2,5 miles (≈4,0 km) — une source alternative mentionne « three-kilometre » pour le seul côté terrestre (**les deux chiffres ne sont pas cohérents, à vérifier**) ; surface enclose ≈133 ha ; hauteur ≈6 m à l'époque romaine, >10 m après ajouts médiévaux | [London Wall, Wikipédia (en)](https://en.wikipedia.org/wiki/London_Wall) | Hauteur et surface recoupées ; longueur incertaine (deux mesures possiblement différentes : linéaire total vs un côté) |
| Portes principales | Oui | Aldgate, Bishopsgate, Newgate, Aldersgate, Cripplegate, Ludgate (6 portes) ; **Moorgate n'est ajoutée qu'en 1415** | — | idem | Recoupé |

#### Monuments et bâtiments

| Élément | Existe en 1340 ? | Dates | Dimensions | Source | Fiabilité |
|---|---|---|---|---|---|
| Tour de Londres — donjon blanc (White Tower) | Oui | Construit par Guillaume le Conquérant à partir de 1078 (≈20 ans de chantier) | — | [White Tower, Wikipédia (en)](https://en.wikipedia.org/wiki/White_Tower_(Tower_of_London)) | Recoupé |
| Tour de Londres — enceinte intérieure | Oui | Créée sous Richard Ier (1189-1199) | — | [Tower of London, Wikipédia (en)](https://en.wikipedia.org/wiki/Tower_of_London) | Recoupé |
| Tour de Londres — murs est/nord, tours de Henri III | Oui | XIIIe siècle, 9 tours construites (2 seulement reconstruites depuis) | — | idem | Recoupé |
| Tour de Londres — enceinte extérieure, nouvelle entrée | Oui, **configuration finale dès 1285** | Remaniement d'Édouard Ier, 1275-1285 ; première utilisation de la brique (Beauchamp Tower) | — | idem | Recoupé — donc en 1340, la Tour est dans l'état achevé en 1285, sans changement majeur ultérieur |
| Old St Paul's — flèche | Oui | Achevée en 1315 ; détruite par la foudre en 1561 | Hauteur la plus citée ≈489 pieds (≈149 m) ; une source évoque une fourchette de 460-489 pieds (140-149 m) | [Old St Paul's Cathedral, Wikipédia (en)](https://en.wikipedia.org/wiki/Old_St_Paul%27s_Cathedral) | Valeur la plus citée, avec marge d'incertitude reconnue par la source elle-même |
| London Bridge — pont de pierre | Oui | Construction 1176-1209 (Peter of Colechurch) | — | [Chapel of St Thomas on the Bridge, Wikipédia (en)](https://en.wikipedia.org/wiki/Chapel_of_St_Thomas_on_the_Bridge) | Recoupé |
| London Bridge — maisons | Oui | Attestées dès 1201 | — | idem | Recoupé |
| London Bridge — chapelle Saint-Thomas | Oui | Achevée en 1209, au centre du pont | Largeur ≈20 pieds (≈6,1 m) | idem | Recoupé |
| Westminster Hall | Oui | Fondations 1097 (Guillaume II le Roux), achevé 1099 | 73 m × 20,7 m (240 × 68 pieds), ≈1547 m², hauteur ≈28 m (92 pieds), murs ≈2 m d'épaisseur | [Westminster Hall, Wikipédia (en)](https://en.wikipedia.org/wiki/Westminster_Hall) | Recoupé — plus grande salle d'Angleterre (et probablement d'Europe) à l'époque |
| Guildhall (bâtiment de 1340, distinct de l'actuel 1411-1440) | Oui, un bâtiment antérieur existe ; **forme précise en 1340 mal connue** | Site utilisé pour des fonctions civiques depuis le début du XIIe siècle ; bâtiment attesté dès 1128 ; cryptes est/ouest actuelles (XIIe s.) antérieures à la reconstruction de 1411 | — | [Guildhall, Medieval London (Fordham)](https://medievallondon.ace.fordham.edu/exhibits/show/medieval-london-sites/guildhall) | **Incertain pour le volume/aspect** — pour VH6, représenter un bâtiment civique modeste sur le même emplacement plutôt que de reprendre l'architecture de 1411-1440 |

#### Population

| Élément | Chiffre | Source | Fiabilité |
|---|---|---|---|
| Londres vers 1340 (avant peste) | « Peut-être jusqu'à 70 000 habitants » (formulation prudente des sources elles-mêmes) | Synthèse de plusieurs pages (Wikipedia, London Museum, History Hit) sur la Peste noire à Londres | Un seul ordre de grandeur ressorti, sans confirmation croisée précise pour 1340 — traiter comme fourchette large 40 000-80 000 |
| Mortalité de la Peste noire (1348-1349) | Estimations dispersées : jusqu'à 30 000 morts sur 70 000 hab. ; ou ≈35 000 morts d'ici 1352 (plus de la moitié de la population intra-muros et faubourgs) ; ou jusqu'aux deux tiers ; ou six sur dix au printemps 1349 | [Consequences of the Black Death, Wikipédia (en)](https://en.wikipedia.org/wiki/Consequences_of_the_Black_Death) ; [The Great Pestilence in London, London Museum](https://www.londonmuseum.org.uk/collections/london-stories/great-pestilence-london/) | Vrai désaccord entre sources — donner la fourchette complète, pas un chiffre unique ; Londres ne retrouve son niveau de population qu'au XVIe siècle |

### 2.3 Orléans

#### Enceintes successives

| Enceinte | Datation | Longueur | Surface intra-muros | Tours / portes | Source | Fiabilité |
|---|---|---|---|---|---|---|
| Enceinte romaine tardive (castrum) | IVe siècle | 2032 m | 25 ha | 5 portes connues ; tours rondes de 8 m de diamètre ; fossé de 10 m de large, 3,5 m de profondeur | Inrap, « Les enceintes d'Orléans depuis l'Antiquité », multimedia.inrap.fr/atlas/orleans | Confirmé (synthèse archéologique) |
| Renforcement en maçonnerie et second fossé | Tournant XIIIe-XIVe siècle | Non chiffré | — | — | Inrap, art. cité | Confirmé |
| Extension vers l'ouest (Bourg Dunois) | Début XIVe siècle — **deux datations rencontrées** : « accrue entre 1300 et 1330 » ou « probablement vers 1356 » ; aucune charte de fondation datée avec précision retrouvée. Le plan VH mentionne « 1345-1350 », qui tombe dans la première fourchette mais n'a pas été retrouvé comme date isolée | Rempart d'≈1000 m ajouté | ≈17 ha ajoutés | — | Storymap MAPO/orleans-metropole.fr ; Inrap ; article Archéopages à consulter en priorité, journals.openedition.org/archeopages/17524 | **Incertain** — la date « 1345-1350 » de VH n'est pas confirmée, à vérifier via l'article Archéopages avant VH0/VH7 |
| Démolition des constructions au pied du rempart | 1359 | — | — | — | Synthèse Inrap : bâtiments détruits en prévision de l'arrivée des Anglais | À confirmer par une seconde source |
| Boulevards (terrassements devant les portes) | À partir de 1404 | — | — | — | Même synthèse Inrap | À confirmer par une seconde source |
| Deuxième enceinte médiévale | Commencée en 1467, achevée symboliquement en 1480 | — | +22,5 ha | — | Inrap, art. cité | Confirmé, mais **postérieur** à 1340-1429 — ne pas confondre avec l'extension du Bourg Dunois |
| Troisième enceinte | Travaux engagés en 1486 | — | — | — | Inrap, art. cité | Confirmé, hors période VH |

**Point de vigilance pour VH7** : au siège de 1428-1429, Orléans est protégée par l'enceinte romaine
renforcée **plus** l'extension du Bourg Dunois du XIVe siècle — la grande extension de 1467-1480
n'existe pas encore. Le plan VH doit utiliser le tracé XIVe siècle, pas celui du XVe.

#### Pont, Tourelles et boulevard

| Élément | Fait | Source |
|---|---|---|
| Construction du pont | Entre 1120 et 1140 (≈20 ans de travaux) ; premier pont de pierre d'Orléans | [Pont des Tourelles, Wikipédia](https://fr.wikipedia.org/wiki/Pont_des_Tourelles) ; recoupé par A. Collin, *Le pont des Tourelles à Orléans (1120-1760)*, [Gallica](https://gallica.bnf.fr/ark:/12148/bpt6k376667h) |
| Longueur totale | 331 m | idem |
| Largeur | 9,75 à 10,40 m selon les sections (arrondi « 10 m » ailleurs) | idem |
| Nombre d'arches à l'origine | 21 (14 entre les Tourelles et la motte Saint-Antoine, 7 entre cette motte et le Châtelet) | idem |
| Nombre d'arches réduit | 18 (13 + 5), après reconstructions | idem |
| Fort des Tourelles | À l'extrémité rive gauche (Sologne) du pont, commandait l'accès sud | idem |
| Châtelet | Forteresse à l'extrémité rive droite du pont, côté ville | idem, recoupé par des pages de vulgarisation |
| Boulevard des Tourelles en 1428 | **Simple terrassement de terre et de bois** (pas encore une fortification de pierre), devant le fort des Tourelles côté Sologne | Wikipédia, « Siège d'Orléans (1428-1429) » ; « Pont des Tourelles » |
| Prise du boulevard | Approche le 12 octobre 1428, pris le 22 octobre 1428 | Confirmé par les deux articles |
| Riposte des Orléanais | Coupure d'une arche du pont côté îlot Saint-Antoine, repli sur la motte Saint-Antoine ; les Anglais coupent leur propre arche ; boulevard de bois construit en hâte au lieu-dit Belle-Croix, où se déroulent les combats de mai 1429 | Wikipédia, « Pont des Tourelles » |
| Reconstruction du boulevard en dur | 1591-1592 (casemates en ravelin à double tenaille) — **hors période VH** | idem |
| Dégâts hivernaux | Débâcle de l'hiver 1434-1435 : dégâts majeurs entre l'îlot et le Châtelet | idem |
| Démolition finale du pont | Juillet 1760 | idem |

**Correction à apporter à VH7** : le plan du chantier mentionne un « boulevard des Tourelles
(1428) » qui pourrait être lu comme une fortification en dur ; les sources indiquent qu'il
s'agissait en 1428 d'un ouvrage provisoire de terre et de bois. La reconstruction en dur n'a lieu
qu'en 1591-1592, bien après la période du jeu. Le modèle VH7 doit représenter un terrassement, pas
une fortification de pierre.

#### Cathédrale Sainte-Croix

| Fait | Date | Détail | Source |
|---|---|---|---|
| Pose de la première pierre | 11 septembre 1287 | Sous l'évêque Gilles Pastai ; plan inspiré d'Amiens mais neuf chapelles rayonnantes au lieu de sept | [Cathédrale Sainte-Croix d'Orléans, Wikipédia](https://fr.wikipedia.org/wiki/Cath%C3%A9drale_Sainte-Croix_d%27Orl%C3%A9ans) |
| Chantier XIVe siècle | Première moitié du XIVe siècle | Chevet achevé par un nouveau chœur ; deux campagnes documentées par un plan sur parchemin conservé à Strasbourg | idem |
| État vers 1340 | — | Chapelles rayonnantes, chœur et déambulatoire achevés ; transept et nef encore en chantier ou non commencés | idem |
| État en 1429 (siège) | — | Seules les chapelles du chevet, autour du chœur, sont debout ; Jeanne d'Arc y assiste à une messe le 2 mai 1429 | idem |
| Reprise des travaux | Seconde moitié du XVe s. (croisée), puis XVIe s. (nef) — hors période VH | — | idem |

**Conclusion pour VH7** : en 1340 comme en 1429, Sainte-Croix est un chantier inachevé limité au
chevet. Ni la nef, ni les tours, ni la façade actuelles n'existent : le modèle doit représenter une
cathédrale « tronquée », chœur gothique seul, sans façade occidentale ni flèche.

#### Faubourgs rasés en 1428

| Fait | Détail | Source |
|---|---|---|
| Période de démolition | Entre le 8 novembre et le 29 décembre 1428 | Wikipédia, « Siège d'Orléans (1428-1429) » |
| Raison | Démolition par les habitants eux-mêmes des faubourgs et bâtiments non protégés, pour ne laisser aucun abri aux assiégeants | idem |
| Collégiale Saint-Aignan | Détruite en 1428 ; déjà démolie une première fois lors des raids anglais de 1358, reconstruite en 1420, donc détruite 8 ans après sa reconstruction | idem |
| Bastilles anglaises | Neuf bastilles construites par les Anglais : sept concentrées au nord-ouest (entre la Loire et la route de Paris), deux isolées à l'est (Saint-Loup, Saint-Jean-le-Blanc) | idem |

#### Population

| Période | Chiffre | Source | Fiabilité |
|---|---|---|---|
| Vers 1340 | **Non trouvé.** Le document de 1328 (F. Lot) donne un total national de 23 671 paroisses et 2 469 987 feux mais aucune valeur par ville n'a été retrouvée pour Orléans dans les extraits en ligne disponibles ; une lecture en texte intégral des deux articles Persée serait nécessaire | F. Lot, 1929, [Persée (I)](https://www.persee.fr/doc/bec_0373-6237_1929_num_90_1_448861), [Persée (II)](https://www.persee.fr/doc/bec_0373-6237_1929_num_90_1_448863) | **Incertain / à compléter** par une lecture directe |
| 1428-1429 (siège) | Environ 20 000 habitants | Wikipédia, « Siège d'Orléans (1428-1429) » | Une seule source — à confirmer (par ex. C. Beaune, *Jeanne d'Arc*) |

Dimensions non trouvées pour Orléans : largeur de la Loire à hauteur de la ville (seul indice
indirect : longueur du pont, 331 m) ; nombre exact de tours et de portes des enceintes du XIVe
siècle (seule l'enceinte romaine a un compte précis).

---

## 3. Parcellaire urbain médiéval typique

### 3.1 Paris

| Donnée | Valeur | Source | Fiabilité |
|---|---|---|---|
| Forme du parcellaire | Parcelles étroites et relativement longues, perpendiculaires à la rue ; maisons construites en profondeur, pignon sur rue, un ou deux étages au-dessus du rez-de-chaussée | B. Bove, « La demeure bourgeoise à Paris au XIVe siècle : bel hôtel ou grant meson ? », *Histoire urbaine*, 2001/1, [Cairn](https://www.cairn.info/revue-histoire-urbaine-2001-1-page-67.htm) ; [Manuel de méthodologie historique, « Demeures urbaines médiévales »](https://manuel-de-methodologie-historique.blog.tudchentil.org/demeures-urbaines-medievales/) | Confirmé qualitativement ; **valeurs numériques précises (mètres) non retrouvées** — l'article Cairn.info n'a pas pu être lu en texte intégral (accès probablement payant) |
| SIG de référence | Le SIG ALPAGE construit des indicateurs géométriques d'allongement des parcelles à partir du parcellaire Vasserot rétro-projeté, mais les valeurs chiffrées synthétiques (largeur/profondeur moyennes) n'ont pas été retrouvées dans les résumés en ligne | alpage.huma-num.fr ; *Paris, de parcelles en pixels. Des plans Vasserot au SIG Alpage*, Presses universitaires de Vincennes | **Incertain** — les données géométriques existent dans le SIG lui-même, pas dans les résumés accessibles par recherche web |

### 3.2 Londres

| Donnée | Valeur | Source | Fiabilité |
|---|---|---|---|
| Référence principale | J. Schofield, *Medieval London Houses*, Yale University Press, 1995 (rééd. 2003) — étude de référence à partir des fouilles archéologiques, documents et plans, de 1200 à 1666 | [Yale University Press](https://yalebooks.co.uk/book/9780300082838/medieval-london-houses/) ; recension *Archaeological Journal*, vol. 153, 1996 | Ouvrage de référence confirmé, **chiffres précis de largeur de façade non extraits** dans cette recherche (texte intégral non disponible en ligne) |
| Encorbellement (jettying) | Technique répandue à Londres : étage supérieur en porte-à-faux au-dessus de la rue | [Jettying, Wikipédia (en)](https://en.wikipedia.org/wiki/Jettying) | Confirmé qualitativement, sans chiffre de largeur associé à Londres spécifiquement |
| Ordre de grandeur anglais transférable | Voir « burgage plots » ci-dessous : façades ≈5-10 m, profondeurs de 15 à plus de 100 m selon la taille de la parcelle d'origine | W. A. Pantin, « Medieval English Town-House Plans », *Medieval Archaeology*, vol. 6-7, [PDF libre, Archaeology Data Service](https://archaeologydataservice.ac.uk/catalogue/adsdata/arch-769-1/dissemination/pdf/vol06-07/6_202_239.pdf) | Article identifié mais non lu en texte intégral dans cette recherche — **à consulter en priorité pour VH4/VH6**, en accès libre |

#### Burgage plots (Angleterre) — ordres de grandeur transférables

| Ville / cas | Largeur de façade | Profondeur | Source |
|---|---|---|---|
| Hungerford (High Street) | ≈11 yards (≈10 m), soit deux perches | RuralHistoria, « What is a Medieval Burgage Plot? » ; Hungerford Virtual Museum |
| Salisbury (parcelles primitives) | 3 perches (≈15,1 m) | 7 perches (≈35,2 m) | burgageplots.info, « A planned approach » |
| Charmouth (Dorset), charte de 1320 | 4 perches (≈20,1 m) | 20 perches (≈100,6 m), soit un demi-acre | burgageplots.info, art. cité |

Conversion utilisée dans ce dossier (non fournie telle quelle par les sources d'origine) :
1 perche (rod/pole anglaise) = 16,5 pieds = 5,0292 m.

**Point de méthode** : ces chiffres viennent de chartes de fondation de villes neuves planifiées,
pas de mesures archéologiques de Londres proprement dite. Ils donnent un ordre de grandeur (façades
10-20 m, profondeurs 35-100 m) plus large que la fourchette 5-8 m / 20-40 m déjà retenue dans le
plan VH pour les cœurs urbains denses. Les parcelles réellement denses des vieux cœurs urbains
(Paris, Londres intra-muros, Orléans) sont probablement plus étroites que ces lots planifiés — à
vérifier avec Schofield et Pantin avant de figer les paramètres du générateur.

### 3.3 Orléans

| Donnée | Valeur | Source | Fiabilité |
|---|---|---|---|
| Seul exemple chiffré trouvé | Maison du XIIe siècle, 8 rue des Gobelets : plan rectangulaire d'≈20 m × 7 m | *Archéologie médiévale*, « Orléans (Loiret). Maisons médiévales et modernes », [OpenEdition](https://journals.openedition.org/archeomed/7216) | Confirmé pour ce bâtiment précis, mais c'est un cas isolé (XIIe siècle), pas une moyenne du XIVe siècle |
| Quartier du Châtelet et rue de la Poterne | Plus de 130 façades à pans de bois, corbeaux sculptés ; activité des drapiers | Pages de vulgarisation touristique | Existence du quartier confirmée, mais **ces façades documentées datent en majorité des XVe-XVIe siècles** (façades « à double lisoir » construites 1490-1520) — à ne pas utiliser tel quel pour l'état de 1340 |
| Toitures | Tuile plate dominante dans le centre historique restauré ; mention qualitative d'ardoise/chaume selon les régions, sans donnée chiffrée propre à Orléans | Pages de vulgarisation ; memoiredubati.fr | **Incertain pour 1340** — les toits visibles aujourd'hui reflètent l'état restauré/moderne |
| Conclusion | Pas de source chiffrée et datée du XIVe siècle spécifique à Orléans trouvée. Recommandation : utiliser par défaut les paramètres du Bassin parisien (proche de Paris, mêmes matériaux disponibles : tuile plate, colombage, torchis) | — | — |

### 3.4 Matériaux de couverture — synthèse générale (toutes villes)

| Fait | Détail | Source | Fiabilité |
|---|---|---|---|
| Évolution générale | Toitures d'abord en chaume ou bardeaux de bois (essentes/tavaillons), remplacées à la demande des autorités municipales par la tuile de terre cuite, notamment à partir des XIIe-XIIIe siècles, pour réduire les risques d'incendie | [BnF, Passerelles, « La couverture »](https://passerelles.essentiels.bnf.fr/fr/chronologie/construction/ffa6bede-f001-4615-a9f3-eadebdbb41db-maison-medievale-urbaine/article/ab1225c8-dfd5-4950-89b1-7a6b57241726-couverture) | Confirmé par une synthèse pédagogique, sans date précise ville par ville |
| Persistance du chaume | Au XVe siècle, à Tours, un quart des maisons sont encore des « chaumières » | Même page BnF | Une seule source — à confirmer |
| Réglementations locales | Rouen, Troyes et Strasbourg édictent des règlements de construction anti-incendie | Même page BnF | Confirmé pour ces villes ; **aucune réglementation propre à Paris, Londres ou Orléans retrouvée avec une date précise** — pour Londres, des ordonnances de couverture limitant le chaume après de grands incendies sont historiquement connues mais non confirmées avec une date exacte dans cette recherche (piste : « London thatch roofing ban 1212 fire ordinance », à vérifier avant VH6) |

### 3.5 Recommandations pour les paramètres du générateur

1. **Façade** : retenir la fourchette 5-8 m déjà indiquée dans le plan VH pour les cœurs urbains
   denses (Paris, Londres, Orléans) plutôt que les 10-20 m des burgage plots anglais, qui concernent
   des villes neuves planifiées à parcelles larges. Documenter cette fourchette comme un choix de
   conception assumé, faute de moyenne chiffrée publiée pour Paris et Orléans.
2. **Profondeur** : 20-40 m est cohérent avec l'ordre de grandeur du seul exemple chiffré retrouvé
   (maison d'Orléans, 20 m), mais cet exemple est isolé — ne pas le présenter comme une moyenne
   validée.
3. **Hauteur** : R+1 à R+2 dans les cœurs denses (confirmé qualitativement par Bove pour Paris),
   avec encorbellement fréquent, en particulier à Londres. Aucune source chiffrée en mètres n'a été
   retrouvée.
4. **Toiture** : tuile plate dominante dans le Bassin parisien (Paris, Orléans) à partir du XIIIe
   siècle sous la pression des règlements anti-incendie ; chaume possible en périphérie/faubourgs
   même en 1340 ; pour Londres, retenir le principe de réglementations anti-chaume sans date
   certaine plutôt que d'inventer une date.
5. **Avant de figer ces paramètres dans le générateur (VH4)**, lire en texte intégral au moins deux
   sources non consultées dans ce dossier : B. Bove, « La demeure bourgeoise à Paris au XIVe
   siècle » (Cairn.info, accès probablement payant ou via bibliothèque universitaire) et W. A.
   Pantin, « Medieval English Town-House Plans » (PDF en accès libre, lien donné en 3.2) — ce
   dernier est gratuit et devrait être lu en priorité.

---

## 4. Recommandations par lot

### VH1 — Forêts

- **Retenir** : ESA WorldCover 2021 (CC BY 4.0, déjà en usage via ZG1) comme couche de base ; BD
  FORÊTS ANCIENNES (IGN, Licence Ouverte Etalab 2.0) comme correctif principal pour distinguer
  forêt ancienne, récente et disparue ; Ancient Woodland Inventory (Natural England, OGL v3) pour
  les provinces anglaises ; KK10 (CC BY 3.0, déjà cité) pour le défrichement, en n'extrayant que les
  tranches temporelles proches de 1340.
- **Écarter** : la vectorisation Cassini du projet Cartofora (WWF/INRA) — aucune licence ni portail
  de téléchargement vérifiable au moment du contrôle. Ne pas chercher de couche SIG dédiée aux
  limites juridiques des Royal Forests anglaises médiévales — s'en tenir aux ellipses déjà présentes
  dans `historical_forests.json`.
- **Risque** : aucun problème de licence bloquant identifié pour ce lot. Le seul risque est de
  perdre du temps à essayer de récupérer les données Cartofora auprès de GIP Ecofor/WWF France sans
  contact direct préalable — à éviter, BD FORÊTS ANCIENNES répond au même besoin plus simplement.

### VH2 — Villages

- **Retenir** : Historical Atlas of the Low Countries (Stapel/KNAW, CC BY 4.0) pour la Flandre et le
  Brabant — vérifier à l'implémentation si la coupe chronologique 1350 est désormais publiée
  (seule la coupe 1500 l'était en 2023), sinon utiliser 1500 en l'assumant comme approximation.
  L'état des paroisses et des feux de 1328 (F. Lot, 1929) pour les valeurs de feux par bailliage,
  en transcrivant à la main les chiffres depuis les articles Persée (domaine public) plutôt qu'en
  reproduisant les scans.
- **Écarter en l'état, ou traiter comme source de contrôle non redistribuable** : Cassini-Geopeuple
  (EHESS) — clause CC BY-NC-SA 3.0 France trouvée sur une page tierce, non reconfirmée sur le site
  source lui-même ; Open Domesday — les données brutes (positions, feux, valeurs) sont en
  CC-NC-BY-SA, contrairement à ce que suppose le plan VH (qui cite CC BY-SA pour l'ensemble). Seules
  les images de folios Open Domesday sont réutilisables commercialement, mais elles n'apportent pas
  les positions/valeurs utiles.
- **Risque bloquant à traiter avant VH2** : sans autorisation écrite d'Open Domesday/Hull ou de
  Cassini-Geopeuple/EHESS, VH2 ne peut pas redistribuer leurs données structurées dans un jeu vendu.
  Alternative : reconstituer indépendamment toponymes et positions approximatives à partir de
  sources en domaine public (texte du Domesday Book du XIe siècle pour l'Angleterre ; communes
  actuelles IGN Admin Express ou OSM pour la France), en utilisant Cassini/Domesday seulement comme
  référence de vérification humaine, sans extraction automatisée en masse. Le recensement de 1328
  ne couvre de toute façon pas la Bretagne, la Bourgogne, la Flandre, la Gascogne anglaise et
  plusieurs apanages : il faudra une méthode de densité par analogie pour ces provinces, quelle que
  soit la source retenue pour le reste de la France.

### VH4 — Tissu urbain générique (parcelles, maisons du kit BR1)

- **Retenir** : les données SIG ALPAGE (parcellaire Vasserot vectorisé, ODbL 1.0) comme source
  principale pour le parcellaire en lanières de Paris — plus riche et sans ambiguïté de licence que
  la simple image de contrôle CC BY 2.0 FR déjà citée dans `paris.json`. Pour les dimensions de
  parcelle (façade/profondeur/hauteur) hors Paris, s'appuyer sur W. A. Pantin (« Medieval English
  Town-House Plans », PDF libre) et sur les burgage plots anglais comme ordre de grandeur
  transférable, en le documentant explicitement comme extrapolation et non comme mesure locale.
- **Écarter** : aucune source chiffrée fiable et datée du XIVe siècle n'a été trouvée pour la
  hauteur des maisons en mètres, ni pour la largeur de façade moyenne à Paris ou à Londres (les
  ouvrages de référence, Bove et Schofield, existent mais leur contenu chiffré n'a pas pu être lu en
  ligne) : ne pas inventer de valeur unique présentée comme mesurée, garder les fourchettes 5-8 m /
  20-40 m comme hypothèse de conception assumée en attendant une lecture en texte intégral.
- **Risque** : le principal risque n'est pas la licence mais la fiabilité — les seules valeurs
  chiffrées vraiment sourcées (burgage plots anglais, une maison d'Orléans) sont soit hors contexte
  géographique, soit un cas isolé. Prévoir une relecture par un historien ou une lecture complète de
  Bove/Schofield/Pantin avant de figer les paramètres numériques du générateur.

### VH5 — Paris

- **Retenir** : les sources déjà citées dans `data/landmarks/paris.json` (plan de Bâle, atlas
  Legrand, contrôle ALPAGE, OSM) restent valables et vérifiées. Ajouter les données SIG ALPAGE
  (ODbL) pour le parcellaire, et les faits datés de la section 2.1 de ce dossier pour ajuster les
  monuments non encore présents en 1340 (tour de l'Horloge, Louvre de Charles V, enceinte de
  Charles V) ou dont l'état est incertain (Grand-Pont, abbaye Saint-Victor).
- **Écarter/encadrer** : Gallica pour tout export haute résolution destiné à devenir un asset du
  jeu (texture, référence de modélisation copiée) — utiliser Gallica seulement pour vérifier des
  faits, jamais pour extraire une image intégrée au jeu, sauf démarche de licence commerciale BnF.
- **Risque non bloquant mais à corriger** : les dimensions du Grand-Pont actuellement cohérentes
  avec `paris.json` (`width_m: 26`) proviennent en réalité du pont reconstruit en 1413, pas de
  l'ouvrage de 1340 — à signaler dans le JSON (commentaire ou champ `note`) lors de la prochaine
  révision du plan, sans nécessairement changer la valeur si aucune source plus ancienne n'est
  trouvée.

### VH6 — Londres

- **Retenir** : Historic England (NHLE, OGL v3) comme source SIG principale et sans risque pour
  recaler les monuments existants ; les faits datés de la section 2.2 pour la Tour de Londres
  (configuration stable depuis 1285), Old St Paul's (flèche ≈149 m depuis 1315), Westminster Hall
  (dimensions précises) et le London Bridge (pont de pierre depuis 1209, maisons, chapelle
  Saint-Thomas).
- **Écarter comme source d'assets, garder comme source de faits** : la carte d'Agas (MoEML), en
  CC BY-NC-SA 4.0 avec une image source dont la reproduction est explicitement interdite par le
  London Metropolitan Archives — clause bloquante pour un jeu vendu si l'image ou ses tuiles sont
  copiées. Utiliser uniquement la toponymie et le tracé des rues qu'elle documente, jamais l'image
  elle-même. Traiter de même le Historic Towns Trust (licence de réutilisation non confirmée) et
  MOLA (aucun portail de données ouvertes identifié).
- **Risque particulier à documenter dans VH6** : le Guildhall actuel (1411-1440) ne doit pas être
  utilisé comme référence pour 1340 — aucune source ne décrit le bâtiment antérieur en détail ;
  représenter un bâtiment civique modeste plutôt que d'anticiper l'architecture du XVe siècle. Les
  chiffres de population et de mortalité de la peste divergent fortement entre sources : présenter
  des fourchettes, pas des valeurs uniques, dans les bulles/codex du jeu.

### VH7 — Orléans

- **Retenir** : les plans anciens d'Orléans sur Gallica (siège de 1428, enceintes, plan de 1778)
  comme sources de *faits* pour la restitution du siège (déjà objet de l'ADR 0026) ; les faits
  datés de la section 2.3 pour l'enceinte romaine (2032 m, 25 ha, chiffres précis), le pont des
  Tourelles (331 m, 21 arches à l'origine), et l'état tronqué de Sainte-Croix (chevet seul, ni nef ni
  façade en 1340 comme en 1429).
- **Écarter/corriger** : ne pas utiliser un export Gallica haute résolution comme asset copié sans
  démarche de licence BnF ; corriger la description actuelle du « boulevard des Tourelles (1428) »,
  qui doit être un terrassement provisoire de terre et de bois, et non une fortification en dur (la
  reconstruction en dur date de 1591-1592, hors période du jeu).
- **Risque à signaler** : la date d'extension du Bourg Dunois utilisée dans le plan VH
  (« 1345-1350 ») n'a pas été retrouvée telle quelle dans les sources consultées — deux fourchettes
  concurrentes existent (1300-1330, ou vers 1356). À trancher en lisant l'article Archéopages cité
  en section 2.3 avant de figer le tracé des enceintes de VH7. La population d'Orléans vers 1340
  n'a pas été trouvée (seul le chiffre de 1428-1429, ≈20 000 habitants, provenant d'une seule
  source) : prévoir une estimation par analogie avec d'autres villes de taille comparable si aucune
  valeur directe n'est trouvée par une lecture complète de F. Lot (1929).

### VH8 — Les six autres villes emblématiques (Rouen, Bordeaux, Avignon, Calais, Bruges)

- Ce dossier ne couvre pas ces villes en détail (hors périmètre demandé). Pour la migration au
  format v2 (VH8), appliquer la même méthode que pour VH5-VH7 : privilégier les fonds en domaine
  public simple sur Wikimedia Commons plutôt que les scans Gallica ou d'institutions patrimoniales
  nationales équivalentes (Bruges : Erfgoedbibliotheek ou KBR belges, à vérifier au cas par cas),
  et distinguer systématiquement une source de faits (citable sans risque) d'une source d'assets
  (à n'utiliser que si la licence autorise explicitement un usage commercial).

---

## Sources de cette recherche

Ce dossier a été produit par recherche web (pages officielles de licence, Wikipédia FR/EN,
Persée, Gallica, sites institutionnels) le 2026-09-25. Les points marqués « incertain » ou « à
vérifier » n'ont pas pu être confirmés par deux sources indépendantes au moment de la rédaction et
devront être revérifiés avant que VH0 ne fige les schémas de données correspondants.
