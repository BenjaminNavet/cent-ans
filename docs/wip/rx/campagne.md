# RX campagne — rendu et lisibilité de la carte

Captures (6 sur 10) : `/private/tmp/claude-501/rx-shots/campagne/{france,normandie,paris_close,parchemin,mediterranee,oural}.png`
(1337 printemps, pas de survol ni de sélection d'armée, pas de test de zoom intermédiaire entre les 6 vues).
`smoke_map.gd` : lancé en headless, n'avait toujours rien affiché après ~12 min (dernière ligne `SimFacade: CampaignSim ready`, pas de « smoke OK ») ; non conclu, à relancer sur machine calme.

## 1. Verdict
- Forces : forêts en volume lisibles de la Normandie à l'Oural ; relief et teintes de sol convaincants ; parchemin très lisible (frontières colorées, rivières, ornements, rose des vents) ; Paris de près riche (cathédrale, îlots, Seine).
- Faiblesse majeure : les maquettes de villes/châteaux (`town_style: maquette`) se rendent en blocs blancs sur-exposés, facettés, qui dominent le paysage à toute hauteur de jeu et jurent avec le sol et la forêt.
- Les nuages (ombres + volumes blancs) cachent de grandes parties de la carte en vue France / Oural, au premier tour par temps clair.
- Dalles blanches froissées autour de Paris : artefact évident en vue rapprochée.
- Étiquettes de villes : lisibles mais deux calques (noms pâles sur forêt, noms rouges anglais) perdent en contraste.

## 2. Constats

### [majeur] [bug/finition] Maquettes blanches sur-exposées, aspect « papier froissé »
**Constat** : en vue régionale (Normandie, Provence, Méditerranée) chaque ville, bourg ou château est une masse blanc cassé facettée sans détail, plus claire que tout le décor ; Caen, Alençon, Montpellier, Aix, Évreux ressortent comme des taches de neige. De près (Paris d=22), des dizaines de « dalles » blanches froissées (jusqu'à 200 px) flottent au premier plan, y compris au milieu de champs, et masquent les terres.
**Preuve** : `normandie.png` (Caen, Alençon, Évreux), `mediterranee.png` (Montpellier, Aix, Toulon), `paris_close.png` (premier plan gauche/bas). Source probable : `game/scripts/map/town_maquette_layer.gd`, `town_maquette_data.gd`, ADR 0158 (GC) ; l'albédo des maquettes n'est pas teinté comme le toit des villes 1:1 (`roofscape.gdshaderinc`). Cause exacte des dalles à confirmer (hameaux/villages maquette ou tuiles F1 `town_far` hors du masque d'enfoncement).
**Correction proposée** : assombrir et teinter l'albédo des maquettes (pierre grise / toit brun, exposition limitée) ; vérifier que le masque `TownFarMask` couvre les hameaux ; ajouter un test de luminance maximale par maquette (capture `--stats`).
**Coût** : M

### [majeur] [conception] Couverture nuageuse trop opaque et trop présente au tour 1
**Constat** : en vue France (d=900) et Oural, 25-40 % de l'écran est recouvert de nuages blancs opaques ; villes, armées et frontières disparaissent dessous (Limoges, Lyon, Lucerne). Pas de lisibilité tactique.
**Preuve** : `france.png`, `oural.png`. Code : `game/scripts/map/campaign_weather_view.gd`, RV-C (`docs/wip/rv-relief-vivant.md` : cumulus relevés).
**Correction proposée** : nuages semi-transparents au-dessus de l'altitude d'étiquettes, ou réduction de l'opacité quand ils recouvrent une armée du joueur / une étiquette ; option « nuages » dans Réglages.
**Coût** : S-M

### [majeur] [finition] Forêts très sombres vers l'horizon, plaines « lunaires »
**Constat** : en Normandie, le bocage du sud est une masse sombre homogène ; au contraire, plaines et landes (Beauce, Normandie est) sont jaune-ocre très saturé, presque orange sur la France d'ensemble, sans parcelles lisibles : le paysage se résume à deux aplats.
**Preuve** : `france.png` (Bassin parisien), `normandie.png`. Réglages : `data/map/colormap_style.yaml`, `terrain.gdshader` (couleur des champs, GC5).
**Correction proposée** : désaturer l'ocre des cultures au dézoom, réintroduire du grain parcellaire (HC4 « variété des champs » toujours ouvert dans `hc-habillage-carte.md`).
**Coût** : M

### [mineur] [finition] Rouge de la faction ennemie très saturé sur le sol (Angleterre)
**Constat** : le territoire anglais est rouge brique uniforme sur forêts et villes ; les arbres y deviennent vert foncé sur rouge (Normandie, haut) : mélange sale. ADR 0155 (EN) voulait du rouge seulement sur frontières/plaques ; c'est ici l'aplat sol.
**Preuve** : `normandie.png` (sud de l'Angleterre), `france.png`.
**Correction proposée** : teinte de faction plus discrète (alpha ou liseré) en vue 3D, aplat conservé au parchemin.
**Coût** : S

### [mineur] [finition] Étiquettes peu contrastées sur forêt
**Constat** : noms de petites villes (Villedieu-les-Poêles, Domfront, Sées, Verneuil) en gris pâle sur vert sombre, à peine lisibles ; le halo n'aide pas assez.
**Preuve** : `normandie.png`.
**Correction proposée** : halo plus épais ou fond semi-opaque sous les étiquettes de rang bas ; masquer ceux qui se recouvrent.
**Coût** : S

### [mineur] [finition] Parchemin : fleuves trop épais et sombres
**Constat** : en vue parchemin les fleuves (Rhin, Danube, Loire…) sont des rubans bleu nuit plus épais que les frontières ; ils écrasent la carte politique (Allemagne, Espagne). Le semis d'arbres sombre mêle aussi les provinces.
**Preuve** : `parchemin.png`.
**Correction proposée** : largeur proportionnelle à l'ordre du fleuve ; teinte plus claire que le bleu des frontières.
**Coût** : S

### [mineur] [finition] Pas de lac ni de côte lisible en Provence à la Méditerranée
**Constat** : côte nette ; mais l'étang de Berre est dessiné (bien) tandis que les marais du Rhône (Camargue) sont de simples aplats. Mer homogène et très grande dans le tiers inférieur : zone vide.
**Preuve** : `mediterranee.png`.
**Correction proposée** : ajouter des routes maritimes visibles / mini-décor (vagues, bateaux ambiants).
**Coût** : M

### [mineur] [conception] Armées : plaques lisibles mais marqueurs minuscules à d=900
**Constat** : à France d=900 les armées sont lisibles (plaque 100/240/620 + cavalier) ; mais les écus de ville à ce niveau remplacent complètement les villes, qui n'ont plus aucune silhouette (cf. `vt.md` : voulu).
**Preuve** : `france.png`.
**Correction proposée** : aucune urgente ; juger en partie réelle.
**Coût** : S

## 3. À ne pas changer
- Forêts généralisées (HC1) et lacs éclaircis (HC2) : volume et lisibilité excellents (Oural, Provence).
- Le sol satellite et le brouillard plafonné (SS) : plus d'effet IGN visible.
- Vue parchemin et ses ornements (rose des vents, sirènes, frontières par posture) : lisibilité très bonne.
- Paris 1:1 de près : cathédrale, ceinture, Seine, ombres.
- Plaques d'armée et écus de ville en vue stratégique.
