# Crédits — Cent Ans

Ce fichier recense les œuvres de tiers utilisées par le jeu et leurs licences. Les assets
produits par les outils du projet (`tools/`) sont signalés comme tels.

Licences du projet : code sous GNU GPL v3.0 ([`LICENSE`](LICENSE)), assets et données originaux sous
CC BY-SA 4.0 ([`LICENSE-ASSETS.md`](LICENSE-ASSETS.md)). Les œuvres de tiers ci-dessous gardent
leur licence propre.

## Icônes — game-icons.net (CC BY 3.0)

Icônes de [game-icons.net](https://game-icons.net) (dépôt
[github.com/game-icons/icons](https://github.com/game-icons/icons)), sous licence
[Creative Commons Attribution 3.0](https://creativecommons.org/licenses/by/3.0/).
**Modifications** : fond noir retiré, recoloration monochrome encre sépia (`#4a3219`),
taille déclarée 64 px (outil `uv run --project tools cent-ans assets icons`). La
correspondance identifiant → fichier → auteur est versionnée dans
`game/assets/icons/icons.json`.

| Auteur | Icônes | Noms (game-icons.net) |
|---|---|---|
| Carl Olsen | 1 | crossbow |
| Caro Asercion | 4 | cloaked-figure-on-horseback, medieval-village-01, spinning-wheel, water-mill |
| Cathelineau | 1 | swordman |
| Delapouite | 60 | abacus, abbot-meeple, bacon, barracks-tent, barrel, bread, broom, castle-ruins, chest-armor, church, coins, diploma, double-fish, farmer, fish-smoking, flag-objective, furnace, graduate-cap, hand-saw, hanging-sign, harbor-dock, healing, herbs-bundle, horse-head, hospital, knight-banner, medicines, medieval-barracks, medieval-pavilion, military-fort, money-stack, palisade, peas, pikeman, plague-doctor-profile, plow, powder-bag, public-speaker, receive-money, rolled-cloth, scroll-quill, shaking-hands, shop, siege-tower, spy, stable, stone-pile, stone-wall, sword-brandish, throne-king, torch, trebuchet, trowel, two-coins, village, well, windmill, wood-pile, wooden-crate, wool |
| Faithtoken | 1 | ore |
| HeavenlyDog | 2 | catapult, defensive-wall |
| Lorc | 84 | angel-wings, anvil, archery-target, armor-vest, arrow-cluster, arrows-shield, bandage-roll, battle-axe, battle-gear, boot-prints, bowman, breastplate, broadsword, cannon, cannon-shot, cash, castle, cauldron, checked-shield, cheese-wedge, cloak-dagger, crested-helmet, crossed-axes, crossed-swords, crown, crown-coin, dove, drama-masks, drop, falling-leaf, fist, flying-flag, galleon, gears, gothic-cross, grapes, gunshot, halberd, hammer-nails, high-shot, holy-symbol, horse-head, hospital-cross, hot-spices, hourglass, laurels, leeching-worm, linked-rings, lyre, metal-bar, muscle-up, open-book, papers, plain-dagger, pocket-bow, potion-ball, powder, prayer, quill-ink, rally-the-troops, round-bottom-flask, scales, scalpel, scalpel-strike, scroll-unfurled, sleepy, snowflake-2, spears, spiked-fence, spiked-mace, sprout, stone-block, stone-spear, stone-tower, sun, swap-bag, target-arrows, thrown-spear, tied-scroll, visored-helm, wax-seal, wheat, wine-glass, wing-cloak |
| Skoll | 4 | mounted-knight, musket, open-treasure-chest, siege-ram |

## Assets tiers (`game/assets/third_party/`)

Chaque dossier contient un `SOURCE.md` (URL, licence, auteur, modifications). Les assets CC0
n'exigent aucune attribution ; ils sont crédités par courtoisie.

### Musique — Kevin MacLeod (incompetech.com), CC BY 4.0 (repli, DA4)

Ces pistes ne servent plus qu'en repli (« fallback » de `data/audio/music.json`, utilisées
seulement si aucune piste d'époque n'est disponible) : trop reconnaissables et anachroniques par
rapport à la musique d'époque de la section suivante. Elles restent créditées et disponibles,
sans avoir été retirées du jeu.

- « Lord of the Land » Kevin MacLeod (incompetech.com). Licensed under Creative Commons: By
  Attribution 4.0 License — http://creativecommons.org/licenses/by/4.0/
- « Village Consort » Kevin MacLeod (incompetech.com). Licensed under Creative Commons: By
  Attribution 4.0 License — http://creativecommons.org/licenses/by/4.0/
- « Crusade » Kevin MacLeod (incompetech.com). Licensed under Creative Commons: By
  Attribution 4.0 License — http://creativecommons.org/licenses/by/4.0/
- « Celtic Impulse » Kevin MacLeod (incompetech.com). Licensed under Creative Commons: By
  Attribution 4.0 License — http://creativecommons.org/licenses/by/4.0/
- « Achaidh Cheide » Kevin MacLeod (incompetech.com). Licensed under Creative Commons: By
  Attribution 4.0 License — http://creativecommons.org/licenses/by/4.0/
- « Angevin B » Kevin MacLeod (incompetech.com). Licensed under Creative Commons: By
  Attribution 4.0 License — http://creativecommons.org/licenses/by/4.0/
- « Procession of the King » Kevin MacLeod (incompetech.com). Licensed under Creative Commons: By
  Attribution 4.0 License — http://creativecommons.org/licenses/by/4.0/
- « Master of the Feast » Kevin MacLeod (incompetech.com). Licensed under Creative Commons: By
  Attribution 4.0 License — http://creativecommons.org/licenses/by/4.0/
- « Minstrel Guild » Kevin MacLeod (incompetech.com). Licensed under Creative Commons: By
  Attribution 4.0 License — http://creativecommons.org/licenses/by/4.0/
- « Moonlight Hall » Kevin MacLeod (incompetech.com). Licensed under Creative Commons: By
  Attribution 4.0 License — http://creativecommons.org/licenses/by/4.0/
- « Teller of the Tales » Kevin MacLeod (incompetech.com). Licensed under Creative Commons: By
  Attribution 4.0 License — http://creativecommons.org/licenses/by/4.0/
- « Suonatore di Liuto » Kevin MacLeod (incompetech.com). Licensed under Creative Commons: By
  Attribution 4.0 License — http://creativecommons.org/licenses/by/4.0/
- « Galway » Kevin MacLeod (incompetech.com). Licensed under Creative Commons: By
  Attribution 4.0 License — http://creativecommons.org/licenses/by/4.0/
- « Thatched Villagers » Kevin MacLeod (incompetech.com). Licensed under Creative Commons: By
  Attribution 4.0 License — http://creativecommons.org/licenses/by/4.0/
- « The Britons » Kevin MacLeod (incompetech.com). Licensed under Creative Commons: By
  Attribution 4.0 License — http://creativecommons.org/licenses/by/4.0/
- « Folk Round » Kevin MacLeod (incompetech.com). Licensed under Creative Commons: By
  Attribution 4.0 License — http://creativecommons.org/licenses/by/4.0/
- « Pippin the Hunchback » Kevin MacLeod (incompetech.com). Licensed under Creative Commons: By
  Attribution 4.0 License — http://creativecommons.org/licenses/by/4.0/

### Musique médiévale et Renaissance — Wikimedia Commons (DA4)

Musique d'époque libre de droits, organisée par culture (bible DA § 9 : France, Angleterre,
Bourgogne/Flandre, Ibérie, Italie) et par contexte (campagne, cour, guerre, menu). Licence vérifiée
page par page (domaine public, CC0, CC BY ou CC BY-SA — jamais NC ni ND).

- Estampie « Retrove », Robertsbridge Codex (Angleterre, XIVe s.) — Metzner, CC BY-SA 3.0.
- « Chominciamento di gioia » (Italie, XIVe s.) — Ririkuku, CC BY-SA 4.0.
- Guillaume Dufay, « Se la face ay pale » — Ensemble Asteria, CC BY-SA 2.5.
- Guillaume Dufay, « Ave Regina caelorum » — enregistrement Commons, CC0.
- Folía d'Ahigal — Loreto Galindo, tamborilero (Fundación Joaquín Díaz), CC BY-SA 3.0.
- Diego Ortiz, Recercadas primera et segunda (*Trattado de Glosas*, 1553) — Phillip W. Serna, CC BY-SA 4.0.
- Cantigas de Santa María (Alphonse X, XIIIe s.), tradition orale castillane — Fundación Joaquín
  Díaz, CC BY-SA 3.0.
- Guillaume de Machaut, « Douce Dame Jolie » et « Riches d'amour et mandians d'amie » (Ars nova,
  XIVe s.), Solage, « Fumeux fume par fumée » (Ars subtilior) — réalisations MIDI, Tetraktys,
  domaine public.
- Francesco Landini, « Ecco la primavera » et « Si dolce non sono » (XIVe s.) — réalisations MIDI,
  Tetraktys, domaine public.
- « Deo gracias Anglia » (Agincourt Carol, anonyme, XVe s.) — réalisation instrumentale, domaine
  public.
- « Sumer is Icumen In » (rota anglaise, XIIIe s.) — Brandtnight2000, CC BY-SA 4.0.
- Gilles Binchois, « Triste plaisir » et « Dueil angoisseux » — réalisations MIDI, Tetraktys,
  CC BY 3.0.
- Démonstration de chalemie (Schalmei), utilisée aussi comme couche de bataille — Ajta,
  CC BY-SA 3.0.

Sources détaillées (URL, licence exacte, traitement) :
`game/assets/third_party/music/wikimedia/SOURCE.md`.

### Couches instrumentales de bataille — Freesound, CC0 1.0 (DA4)

Musique de bataille en couches superposables (tambour, trompette droite, bourdon de cornemuse) au
travers de `BattleMusicDirector` (`data/audio/battle_layers.json`) : enregistrements CC0
(licence vérifiée page par page), coupés en boucles courtes, normalisés à -16 LUFS. Détail :
`game/assets/audio/music/battle_layers/SOURCE.md`.

### Polices — SIL Open Font License 1.1

- **EB Garamond** — Georg Duffner, Octavio Pardo (The EB Garamond Project Authors),
  [fonts.google.com/specimen/EB+Garamond](https://fonts.google.com/specimen/EB+Garamond).
- **IM FELL English** — Igino Marini ([iginomarini.com](http://www.iginomarini.com)), nom
  réservé « IM FELL ».

Le texte de la licence (`OFL.txt`) accompagne chaque police.

### Modèles 3D — CC0 1.0

- **Quaternius** ([quaternius.com](https://quaternius.com)) : personnages riggés de l'Ultimate
  Modular Characters Pack (King, Adventurer, Hooded Adventurer, Farmer), chevaux et âne de
  l'Ultimate Animated Animal Pack, Medieval Village Pack (39 bâtiments et props) — fichiers
  obtenus via [Poly Pizza](https://poly.pizza). Les figurines de bataille skinnées
  (`game/assets/models/battle_skinned/`, lot V2) en dérivent : pièces recolorées, habillées
  d'équipement procédural, décimées, animations rééchantillonnées et complétées.
- **Kenney** ([kenney.nl](https://kenney.nl)) : Castle Kit.
- **Poly Haven** ([polyhaven.com](https://polyhaven.com)) : Fir Tree 01, Pine Tree 01, Grass
  Medium 01 (Rico Cilliers, Rob Tuytel), Grass Medium 02 (Rico Cilliers). Modifiés : LOD2
  seulement, décimation, matériaux simplifiés.

### Matériaux des villes emblématiques — CC0 1.0

- **Poly Haven** ([polyhaven.com](https://polyhaven.com)) : Medieval Blocks 03, Castle Wall
  Varriation, Medieval Red Brick, Roof Slates 02, Clay Roof Tiles 02, Roof Tiles 14, Clay Plaster,
  Old Planks 02, Reed Roof 04, Cobblestone Floor 08 (1k). Ramenées à des cartes de détail et
  assemblées en atlas avec des couches procédurales (pans de bois, plomb, vitrail, vieillissement) :
  `game/assets/textures/landmarks/` (lot L3, détail dans son `SOURCE.md`).

### Textures du terrain de campagne — CC0 1.0

- **Poly Haven** ([polyhaven.com](https://polyhaven.com)) : Aerial Grass Rock, Aerial Mud 1,
  Forrest Ground 01, Aerial Rocks 01, Sparse Grass, Snow Field Aerial, Aerial Beach 01 (1k),
  téléchargées par `tools/cent_ans_tools/geo/textures.py` et réduites en albédo + normale/rugosité
  (`game/assets/textures/terrain/`).

### Ciels HDRI — CC0 1.0

- **Belfast Open Field** — Dimitrios Savva, Jarod Guest (Poly Haven).
- **Autumn Field Pure Sky** — Sergej Majboroda, Jarod Guest (Poly Haven).
- **Kloofendal 48d Partly Cloudy, Kloofendal Misty Morning, Kloofendal Overcast** (Pure Sky) — Greg Zaal,
  Jarod Guest (Poly Haven).
- **Overcast Soil, Snow Field** (Pure Sky) — Sergej Majboroda, Jarod Guest (Poly Haven).
- **Syferfontein 18d Clear, Kloppenheim 06** (Pure Sky) — Greg Zaal, Jarod Guest (Poly Haven).

### Interface — CC0 1.0

- **Fantasy UI Borders** — Kenney ([kenney.nl](https://kenney.nl)).
- **Parchment GUI** — zwonky ([OpenGameArt](https://opengameart.org/content/parchment-gui)).

### Sons de bataille et ambiances — Freesound, CC0 1.0

Banque AU1 (`game/assets/audio/battle/`, `game/assets/audio/ambience/`) : enregistrements
[Freesound](https://freesound.org) sous licence CC0 (vérifiée page par page), découpés, mélangés
et bouclés par `tools/cent_ans_tools/audio_bank.py`. Détail fichier par fichier (numéro, titre,
lien, traitement) dans `game/assets/audio/SOURCE.md`. Merci aux auteurs : 6polnic, adharca,
AlanCat, Ali_6868, Archeos, AyaDrevis, bajko, Blankened, bolkmar, bruno.auzet, Christopherderp,
craigsmith, DeadVDI, Defelozedd94, DeVern, DigestContent, DigPro120, DRFX, ethanchase7744,
FillMat, florianreichelt, foxen10, freefire66, greyfeather, Ittaisha, iwanPlays, jackstraton,
jamesdrake89, JoeDinesSound, JohnBuhr, joseppujol, juryduty, kasparsj, Kinoton, Kubuzz,
loopbasedmusic, Lucas_Schacht, modusmogulus, Mythmazter, nekoninja, omerbhatti34, pborel,
PixelsphereStudios, PorkMuncher, qubodup, Quickmusik, SamuelGremaud, saturdaysoundguy,
shadoWisp, Simonus18, spycrah, Twisted_Euphoria, unfa, waxsocks, WelvynZPorterSamples, xkeril.

Les sons d'interface (`game/assets/audio/ui/`, lot UB1) sont découpés hors ligne dans ces mêmes
fichiers et dans les effets procéduraux du projet par `tools/cent_ans_tools/ui_sounds.py` ;
détail dans `game/assets/audio/ui/SOURCE.md`.

## Données géographiques

Le relief et l'hydrographie de la carte de campagne sont des produits dérivés, calculés par les
outils du projet (`tools/cent_ans_tools/geo`, `docs/geo.md`) à partir des sources ci-dessous ; le
cache du relief fin (`data/map/pyramid/`, livré dans l'application ou dans le dossier « Cent Ans
relief », ADR 0036) en fait partie. Les mentions d'attribution exigées par chaque licence sont
reproduites telles quelles (entre guillemets).

- **Relief (terre et bathymétrie)** : ETOPO 2022 15 Arc-Second Global Relief Model, NOAA
  National Centers for Environmental Information — domaine public (données du gouvernement des
  États-Unis). Citation : *NOAA National Centers for Environmental Information. 2022: ETOPO
  2022 15 Arc-Second Global Relief Model. doi:10.25921/fd45-gt74*.
- **Terres, côtes, rivières, lacs** : [Natural Earth](https://www.naturalearthdata.com/)
  (10 m physical) — domaine public. « Made with Natural Earth. »
- **Relief fin (terres de l'emprise jouable)** : Copernicus DEM GLO-90, © DLR e.V. 2010-2014 et
  © Airbus Defence and Space GmbH 2014-2018, fourni dans le cadre de COPERNICUS par l'Union
  européenne et l'ESA — tous droits réservés ; licence gratuite avec attribution. Tuiles lues
  sur le bucket public AWS Open Data `copernicus-dem-90m` (lot R1, ADR 0019). Mention : « © DLR
  e.V. 2010-2014 and © Airbus Defence and Space GmbH 2014-2018 provided under COPERNICUS by the
  European Union and ESA; all rights reserved. » Sert aussi aux étages E1-E2 de la pyramide de
  relief (palier 1, lot ZG1) et aux horizons des batailles (`game/assets/horizon/relief/`, EP2).
- **Relief rapproché (pyramide de relief, palier 2)** : Copernicus DEM GLO-30 Public, © DLR e.V.
  2010-2014 et © Airbus Defence and Space GmbH 2014-2018, fourni dans le cadre de COPERNICUS par
  l'Union européenne et l'ESA — tous droits réservés ; licence gratuite avec attribution
  (conditions : https://dataspace.copernicus.eu/explore-data/data-collections/copernicus-contributing-missions/collections-description/COP-DEM).
  « Copernicus Digital Elevation Model (DEM) was accessed on 2026-09-25 from
  https://registry.opendata.aws/copernicus-dem. » Les organismes en charge du programme
  Copernicus n'encourent aucune responsabilité pour l'usage qui en est fait. Modifié : canopée,
  bâti moderne et retenues de barrages retirés (lot ZG1, ADR 0036). Les tuiles E1-E2 de la
  pyramide dérivent de GLO-90 (même licence).
- **Occupation du sol actuelle (correction du relief)** : ESA WorldCover 10 m 2021 v200,
  © ESA WorldCover project 2021 / Contains modified Copernicus Sentinel data (2021) processed by
  ESA WorldCover consortium — licence CC BY 4.0. Citation : *Zanaga, D. et al. (2022). ESA
  WorldCover 10 m 2021 v200. doi:10.5281/zenodo.7254221* ; accédé le 2026-09-25 depuis
  https://registry.opendata.aws/esa-worldcover-vito. Sert uniquement à retirer arbres, bâti et
  plans d'eau modernes du relief GLO-30 (lot ZG1).
- **Défrichement vers 1340** : KK10 Anthropogenic Land Cover Change — Kaplan, J. O. et
  Krumhardt, K. M. (2017), PANGAEA, doi:10.1594/PANGAEA.871369, licence CC BY 3.0 ; méthode :
  Kaplan et al. (2011), *The Holocene* 21(5), doi:10.1177/0959683610386983. Moyenne 1330-1349,
  combinée aux grandes forêts et zones humides nommées de `data/map/historical_forests.json` et
  `data/map/wetlands.json` (sources par entrée).
- **Relief détaillé des zones historiques (palier 3, lot ZG3, ADR 0036)** — modèles numériques
  de terrain sans sursol, rééchantillonnés à 11, 5,6 et 2,8 m (34 zones de
  `data/map/detail_zones.json`, étages E5-E7) :
  - France : RGE ALTI® 1 m / 5 m, © IGN (Institut national de l'information géographique et
    forestière), [Licence Ouverte Etalab 2.0](https://www.etalab.gouv.fr/licence-ouverte-open-licence/) ;
    service WMS-R de la Géoplateforme (`data.geopf.fr`). Mention : « Source : IGN – RGE ALTI® ».
  - Angleterre : LIDAR Composite Digital Terrain Model (DTM) 1 m, Environment Agency,
    [Open Government Licence v3.0](https://www.nationalarchives.gov.uk/doc/open-government-licence/version/3/).
    Mention : « © Environment Agency copyright and/or database right 2022. All rights reserved. »
  - Pays-Bas : Actueel Hoogtebestand Nederland (AHN) DTM 0,5 m, Rijkswaterstaat / Het Waterschapshuis,
    service PDOK, [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/) (aucune attribution
    exigée ; citée par courtoisie).
  - Flandre : Digitaal Hoogtemodel Vlaanderen II (DTM 1 m) et I (DTM 5 m), © Digitaal Vlaanderen,
    [Modellicentie Gratis Hergebruik v1.0](https://data.vlaanderen.be/doc/licentie/modellicentie-gratis-hergebruik/v1.0)
    (réutilisation gratuite, y compris commerciale, avec mention de la source).
  - Repli (Tournai, Wallonie : pas de service à valeurs brutes) : Copernicus DEM GLO-30, © DLR e.V.
    2010-2014 et © Airbus Defence and Space GmbH 2014-2018, fourni dans le cadre de COPERNICUS par
    l'Union européenne et l'ESA ; licence gratuite avec attribution.
  - Effacement des aménagements modernes (autoroutes, voies ferrées, carrières, retenues, digues de
    port) : masques calculés à partir d'OpenStreetMap, © les contributeurs d'OpenStreetMap,
    [ODbL 1.0](https://opendatacommons.org/licenses/odbl/1-0/) — méthode seulement : aucune donnée
    OSM n'est redistribuée (masques intermédiaires hors dépôt).
- **Routes** (`data/map/roads.geojson`) : Itiner-e, *A High-Resolution Dataset of Roads of the
  Roman Empire* (de Soto, Pažout, Brughmans et al.), Zenodo doi:10.5281/zenodo.17122148, licence
  CC BY 4.0 ; citation : *de Soto P., Pažout A., Brughmans T. et al. (2025). Itiner-e: A
  high-resolution dataset of roads of the Roman Empire. Scientific Data.
  doi:10.1038/s41597-025-06140-z*. Complété par des tronçons calculés par le projet.
- **Hameaux** (`data/map/hamlets.json`) : [GeoNames](https://www.geonames.org/) `cities500`,
  © GeoNames, licence CC BY 4.0.
- **Villes emblématiques** (`data/landmarks/`) : quelques dizaines de points de contrôle de position
  par ville relevés à la main sur © les contributeurs
  d'[OpenStreetMap](https://www.openstreetmap.org/copyright) (ODbL 1.0), arrondis. Les plans
  anciens de Wikimedia Commons cités dans ces fichiers ont servi de référence et ne sont pas
  redistribués.
- **Réseau hydrographique fin (lot ZG5a, ADR 0036)** — tracés recalés sur la pyramide de relief,
  canaux postérieurs à 1340 retirés :
  - France : BD TOPAGE® 2025, tronçons hydrographiques (IGN, OFB, agences de l'eau ; diffusion
    SANDRE, `services.sandre.eaufrance.fr`),
    [Licence Ouverte Etalab 2.0](https://www.etalab.gouv.fr/licence-ouverte-open-licence/).
    Mention : « Source : BD TOPAGE® – IGN, OFB ».
  - Grande-Bretagne : OS Open Rivers, Ordnance Survey,
    [Open Government Licence v3.0](https://www.nationalarchives.gov.uk/doc/open-government-licence/version/3/).
    Mention : « Contains OS data © Crown copyright and database right 2026. »
  - Bénélux, Rhénanie, versants suisse, italien et espagnol du cœur : EU-Hydro River Network
    Database v1.3, Copernicus Land Monitoring Service, Agence européenne pour l'environnement,
    lue sur le service ArcGIS public `image.discomap.eea.europa.eu` ; accès libre, complet et
    gratuit selon la politique de données Copernicus (règlement délégué (UE) n° 1159/2013).
    Mention : « © European Union, Copernicus Land Monitoring Service 2020, European Environment
    Agency (EEA). »
  - Hors cœur : Natural Earth (voir ci-dessus).
- Traitement (reprojection EPSG:3035, découpage des provinces) : outils `tools/cent_ans_tools/geo`
  (voir `docs/geo.md`).

## Données historiques

Personnages, provinces, factions, technologies, unités et bâtiments de `data/` sont rédigés
pour le projet à partir de sources publiques ; chaque fichier liste ses sources dans le champ
`sources` (titres d'articles de Wikipédia en français ou références d'ouvrages). Les textes de
Wikipédia ne sont pas recopiés.

## Enluminures du domaine public (`game/assets/art/`, lot AR1)

Écrans de chargement, vignettes d'événements et écrans de fin : enluminures des XIVe-XVe siècles,
**domaine public** (œuvres anonymes ou d'artistes morts depuis plus de 500 ans ; numérisations de
la BnF/Gallica, de la British Library et d'autres fonds, publiées sur Wikimedia Commons sous la
mention « Public domain » ou CC0). Recadrées et réduites par `tools/cent_ans_tools/art_plates.py`
(déclarations et recadrages dans `data/ui/illustrations.json`).

- `ld_crecy` : Jean Froissart, Chroniques, BnF, ms. Français 2643, f. 165v — Loyset Liédet ([Wikimedia Commons](https://commons.wikimedia.org/wiki/File%3ABattle_of_crecy_froissart.jpg)).
- `ld_poitiers` : Jean Froissart, Chroniques, BnF, ms. Français 2643 — Loyset Liédet ([Wikimedia Commons](https://commons.wikimedia.org/wiki/File%3ABattle-poitiers%281356%29.jpg)).
- `ld_najera` : Jean Froissart, Chroniques, BnF, ms. Français 2643, f. 312v — Loyset Liédet ([Wikimedia Commons](https://commons.wikimedia.org/wiki/File%3ABattle_najera_froissart.jpg)).
- `ld_auray` : Jean Froissart, Chroniques, BnF, ms. Français 2643 — Loyset Liédet ([Wikimedia Commons](https://commons.wikimedia.org/wiki/File%3ABattle_of_Auray_2.jpg)).
- `ld_calais` : Jean Froissart, Chroniques, BnF, ms. Français 2643 — Loyset Liédet ([Wikimedia Commons](https://commons.wikimedia.org/wiki/File%3AThe_French_attempt_to_recapture_Calais_from_England_%281350%29.jpg)).
- `ld_hennebont` : Jean Froissart, Chroniques, BnF, ms. Français 2643, f. 104v — Loyset Liédet ([Wikimedia Commons](https://commons.wikimedia.org/wiki/File%3ASi%C3%A8ge_d%27Hennebont.jpg)).
- `ld_reims` : Jean Froissart, Chroniques, BnF, ms. Français 2643 — Loyset Liédet ([Wikimedia Commons](https://commons.wikimedia.org/wiki/File%3AEdouard_III_assi%C3%A8geant_Reims.jpg)).
- `ld_sluys` : Jean Froissart, Chroniques, BnF, ms. Français 2643, f. 72r — Loyset Liédet ([Wikimedia Commons](https://commons.wikimedia.org/wiki/File%3ABattleofSluys.jpeg)).
- `ld_la_rochelle` : Jean Froissart, Chroniques, BnF, ms. Français 2643 — Loyset Liédet ([Wikimedia Commons](https://commons.wikimedia.org/wiki/File%3ABataille_de_la_Rochelle.jpg)).
- `ld_paris` : Très Riches Heures du duc de Berry, juin, musée Condé, Chantilly, ms. 65, f. 6v — Frères de Limbourg ([Wikimedia Commons](https://commons.wikimedia.org/wiki/File%3ALes_Tr%C3%A8s_Riches_Heures_du_duc_de_Berry_juin.jpg)).
- `vg_plague` : Chroniques de Gilles Li Muisis, Bibliothèque royale de Belgique, ms. 13076-77, f. 24v — Pierart dou Tielt (attribué) ([Wikimedia Commons](https://commons.wikimedia.org/wiki/File%3ABurying_Plague_Victims_of_Tournai.jpg)).
- `vg_revolt` : Jean Froissart, Chroniques, BnF, ms. Français 2643 (la Jacquerie à Meaux, 1358) — Loyset Liédet ([Wikimedia Commons](https://commons.wikimedia.org/wiki/File%3AJacquerie_meaux.jpg)).
- `vg_succession` : Grandes Chroniques de France, British Library, Royal 20 C VII, f. 216 (sacre de Charles VI) — Anonyme ([Wikimedia Commons](https://commons.wikimedia.org/wiki/File%3ASacre_Charles6_France_01.jpg)).
- `vg_death` : Martial d'Auvergne, Vigiles de Charles VII, BnF, ms. Français 5054, f. 244v — Anonyme ([Wikimedia Commons](https://commons.wikimedia.org/wiki/File%3AVigiles_du_roi_Charles_VII-12.jpg)).
- `vg_marriage` : Jean Froissart, Chroniques, BnF, ms. Français 2646, f. 245v — Anonyme ([Wikimedia Commons](https://commons.wikimedia.org/wiki/File%3ARichard_II_et_Isabelle_de_France_en_1396.jpg)).
- `vg_birth` : Martial d'Auvergne, Vigiles de Charles VII, BnF, ms. Français 5054 (baptême du futur Charles VII) — Anonyme ([Wikimedia Commons](https://commons.wikimedia.org/wiki/File%3AVigiles_du_roi_Charles_VII_00.jpg)).
- `vg_ransom` : Jean Froissart, Chroniques, bibliothèque municipale de Besançon, ms. 864, f. 172 (capture de Jean II) — Maître de Giac ([Wikimedia Commons](https://commons.wikimedia.org/wiki/File%3ABataille_de_Poitiers_-_BM_Besan%C3%A7on_Ms864_f172.jpg)).
- `vg_diplomacy` : Grandes Chroniques de France, BnF, ms. Français 2813, f. 357v (hommage d'Amiens, 1329) — Anonyme ([Wikimedia Commons](https://commons.wikimedia.org/wiki/File%3AHomage_d%27Edouard_III.jpg)).
- `vg_war` : Jean Froissart, Chroniques, British Library, Harley 4380, f. 84 — Anonyme ([Wikimedia Commons](https://commons.wikimedia.org/wiki/File%3AThe_King_of_Hungary_holding_council_in_his_tent_on_the_battlefield_-_Froissart%27s_Chronicles_%28Volume_IV%2C_part_2%29_%281470-1475%29%2C_f.84_-_BL_Harley_MS_4380.jpg)).
- `vg_battle` : Jean Froissart, Chroniques, vers 1410 (Édouard III fait compter les morts de Crécy) — Maître de Virgile ([Wikimedia Commons](https://commons.wikimedia.org/wiki/File%3AEdward_III_counting_the_dead_on_the_battlefield_of_Cr%C3%A9cy.jpg)).
- `vg_siege` : Jean Froissart, Chroniques, British Library, Royal 18 E I, f. 345 — Anonyme ([Wikimedia Commons](https://commons.wikimedia.org/wiki/File%3ACapture_of_Wark_Castle_-_Froissart%2C_Chroniques_de_France_et_d%27Angleterre%2C_Book_II_%28c.1460-1480%29%2C_f.345_-_BL_Royal_MS_18_E_I.jpg)).
- `vg_church` : Couronnement de Clément VII (1378), archives iconographiques du palais du Roure, Avignon — Anonyme ([Wikimedia Commons](https://commons.wikimedia.org/wiki/File%3ACouronnement_Cl%C3%A9ment_VII.jpg)).
- `vg_intrigue` : Jean Froissart, Chroniques, British Library, Royal 18 E I, f. 172 — Anonyme ([Wikimedia Commons](https://commons.wikimedia.org/wiki/File%3AMurder_of_Simon_Sudbury_-_Froissart%2C_Chroniques_de_France_et_d%27Angleterre%2C_Book_II_%28c.1460-1480%29%2C_f.172_-_BL_Royal_MS_18_E_I.jpg)).
- `end_battle_victory` : Jean Froissart, Chroniques, BnF, ms. Français 2643 (bataille de Rosebecque) — Loyset Liédet ([Wikimedia Commons](https://commons.wikimedia.org/wiki/File%3ASlagbijrozebeke.jpg)).
- `end_battle_defeat` : Martial d'Auvergne, Vigiles de Charles VII, BnF, ms. Français 5054, f. 11 — Anonyme ([Wikimedia Commons](https://commons.wikimedia.org/wiki/File%3AVigiles_du_roi_Charles_VII_06.jpg)).
- `end_campaign_victory` : Martial d'Auvergne, Vigiles de Charles VII, BnF, ms. Français 5054 (Charles VII devant Tartas) — Anonyme ([Wikimedia Commons](https://commons.wikimedia.org/wiki/File%3AVigiles_du_roi_Charles_VII_14.jpg)).
- `end_campaign_defeat` : Martial d'Auvergne, Vigiles de Charles VII, BnF, ms. Français 5054 (funérailles du duc François Ier de Bretagne) — Anonyme ([Wikimedia Commons](https://commons.wikimedia.org/wiki/File%3AVigiles_du_roi_Charles_VII_36.jpg)).

Images générées pour combler les manques (`ld_avignon`, `vg_famine`, `vg_treasury`, `vg_trade`) : OpenRouter
(`openai/gpt-5-image-mini`), dépenses consignées dans `docs/budget.md` (session 7).

## Assets produits par le projet

- **Écus** (`game/assets/heraldry/`) : dessinés procéduralement (Pillow) à partir des blasons
  de `data/factions/`.
- **Modèles 3D** (`game/assets/models/`) : générés par scripts Blender (`tools/blender_scripts/`).
- **Sons et musiques** (`game/assets/audio/sfx/`, `game/assets/audio/music/`) : synthèse
  procédurale (numpy/scipy), sans échantillon externe. Les sons de bataille et ambiances
  (`battle/`, `ambience/`) viennent de Freesound (CC0, voir plus haut).
  Exception : l'huile bouillante des sièges (`battle/boiling_oil_*.ogg`, lot SG2) est une
  synthèse procédurale (`tools/cent_ans_tools/sg2_sounds.py`).
- **Engins de siège** (`game/assets/models/siege/`, lot SG2) : trébuchet, mangonneau, bombarde,
  bélier et beffroi modélisés par script Blender (`tools/blender_scripts/siege_engines.py`),
  textures de bois Poly Haven (CC0) déjà créditées.
- **Portraits** (`game/assets/portraits/`) : images générées par IA via OpenRouter
  (`openai/gpt-5-image-mini`), dépenses consignées dans `docs/budget.md`.
- **Voix** (`game/assets/audio/voice/` : répliques des unités, discours des généraux,
  conseiller) : voix **générées par synthèse vocale** (OpenAI `gpt-4o-mini-tts`), à partir des
  textes du projet (`data/voice/`, `data/speeches/`) ; aucune voix d'acteur. Outil
  reproductible `tools/cent_ans_tools/voice_tts.py` (liste des fichiers, voix et coût dans
  `game/assets/audio/voice/manifest.json`), dépenses consignées dans `docs/budget.md`.

## Polices

- Interface parchemin : polices serif du système (Georgia, Palatino, Times New Roman), non
  redistribuées avec le jeu.
- Étiquettes de carte et textes sans thème : police par défaut intégrée à Godot (Open Sans,
  SIL Open Font License 1.1).

## Logiciels

- [Godot Engine](https://godotengine.org) 4.7 — licence MIT.
- [godot-rust (gdext)](https://github.com/godot-rust/gdext) — licence MPL 2.0.
- Bibliothèques Rust et Python : voir `core/Cargo.lock` et `tools/uv.lock` (licences
  permissives MIT / Apache 2.0 / BSD).
