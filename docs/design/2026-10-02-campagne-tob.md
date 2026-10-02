# TB — Carte de campagne façon Thrones of Britannia

Date : 2026-10-02. Statut : validé par le joueur le 02/10 (plan, ordre des vagues, enveloppe fal.ai de 25 $). Suivi : `docs/wip/tb-campagne-tob.md`.
Budget : **0 $, sans fal.ai (ADR 0152, décision du joueur le 02/10, remplace l'ADR 0149)**. Les mentions
de fal.ai et les coûts ci-dessous sont caducs : TB0 sans repeints, TB3 et TB5 avec les kits et textures
existants et Blender, TB7 reporté.

## 0. Demande et cadrage

Demande du joueur (02/10) : rendre le jeu « beaucoup plus joli ». Ses arbitrages :
1. Référence principale : **Total War Saga: Thrones of Britannia** (ToB). C'est cohérent avec la
   bible DA (§ 0 et § 1 : « Thrones of Britannia / Attila pour la retenue »), qui ne change donc pas.
2. Priorité : **carte de campagne**. Les batailles et les écrans hors campagne sont hors périmètre.
3. Dépenses : dépassement du plafond de 50 $ accepté **uniquement sur fal.ai** (ADR 0149).
4. Références visuelles à récupérer en ligne (§ 1).

## 1. Ce que fait Thrones of Britannia (références)

Le réseau du conteneur cloud du 02/10 refuse Steam, ArtStation, Wikimedia, les sites de presse et
fal.ai. La recherche n'a donc donné que du **texte**, et aucune image n'a été rapatriée. Il faudra
une session locale ou une politique réseau élargie pour la planche (lot TB0).

Relevé (sources en fin de document) :
- **Une carte vivante.** PCGamesN : « a thing of beauty, alive with rippling grass, criss-crossing
  caravans, and Viking longships harassing the coast ». La carte est le cœur du jeu (« the meat of
  the game is in the map and the menus »).
- **Des saisons visibles.** La neige recouvre les îles en hiver. Elle compte aussi dans les règles :
  l'attrition des marches d'hiver augmente au pays de Galles.
- **Des bâtiments visibles qui grandissent.** Les colonies mineures (village, marché, mine, ferme,
  abbaye) n'ont pas de murs et ont 1-2 emplacements ; leur type dépend du terroir. Les bâtiments
  construits « apparaissent sur la carte et grandissent avec leurs améliorations ». Les capitales
  de province ont des murs.
- **Le relief réel.** La carte est faite à partir de données de terrain réelles, 23 fois plus
  grande que la Bretagne d'Attila.
- **Une identité d'époque.** L'interface et les cartes d'unité s'inspirent de l'art insulaire.
  Pour Cent Ans, l'équivalent est l'enluminure gothique parisienne (bible § 5).
- **Une vue stratégique en parchemin.** Héritée de Rome II et Attila : au dézoom maximal, la carte
  devient un parchemin peint sans fouillis 3D. Cent Ans l'a déjà (ADR 0124).

À confirmer sur images (TB0) : l'aspect exact du brouillard de guerre, le style des étiquettes,
la densité des pictogrammes.

## 2. Écarts avec l'existant

Sources : inventaire du code du 02/10 et capture `docs/img/readme/campagne.jpg` (30/09).

| ToB | Cent Ans aujourd'hui | Écart |
|---|---|---|
| Hiver enneigé, automne, moissons visibles de loin | Les saisons existent (`campaign_season`, CV1 : champs, vignes, forêts, ligne de neige). Mais la **carte de couleur satellite** (`satellite_ground.gdshaderinc:22-28`, ADR 0142) remplace la teinte saisonnière à 100 % au zoom large et moyen, et à 45 % de près. En pratique, seule la neige se voit, et seulement de près. Ni la mer ni la lumière ne changent avec la saison. | **fort, et facile à corriger** |
| Bâtiments construits visibles, qui grandissent | Villes 1:1 tirées de données de 1340, figées (ADR 0138). `replace_models` est vide et `model_holder()` renvoie null (`settlement_layer.gd:1614, 1643`). La croissance CV1 est calculée mais n'est pas affichée. Pas de murs ajoutés quand on fortifie. Aucune ferme, mine ou abbaye hors les murs. Un chantier = un « ⚒ » en Label3D. | **fort** |
| Carte vivante (herbe, caravanes, drakkars) | Déjà là, surtout au palier proche : fumées, moulins, oiseaux, charrettes FK3, paysans saisonniers, navires. L'herbe du terrain ne bouge pas (seul le « clutter » bouge). | faible |
| Ruines et ravages | Champs brûlés (`terroir_burn`), hameaux brûlés, camps de siège. La suie sur les villes 1:1 passe par le support null, donc elle ne s'affiche probablement pas. La peste n'a pas de marqueur. | moyen |
| Image sobre et lisible | Capture du 30/09 : nuages et brumes sur une grande partie de l'écran en vue moyenne ; Angleterre délavée par le brouillard de guerre ; pictogrammes nombreux (rosaces rouges, étoiles) ; tracés rouges vifs (chemin d'armée ou frontières FR1) ; noms tronqués en haut d'écran ; aucun nom de province en vue normale. | moyen |
| Mer du Nord grise l'hiver, côtes à falaises | Eau correcte (profondeur, reflet, écume) mais sans saison ni tempête, sans falaises (Douvres, pays de Caux), reflets spéculaires seulement. | moyen |
| Lumière dorée, étalonnage d'époque | AgX, SSAO, brouillard de distance. Pas d'étalonnage par saison, pas de SSIL ni de brouillard volumétrique. | moyen |
| Vue stratégique en parchemin | Déjà riche (ADR 0124 : hachures, vignettes à l'encre, roses des vents, monstres marins). | faible |

## 3. Lots

Les règles de jeu restent dans `core/` (CLAUDE.md). Ces lots touchent au rendu, sauf indication.
Toute nouvelle correspondance (bâtiment → maquette, saison → teinte) va dans `data/` avec son schéma.
Vérification : tests et `smoke.gd` d'abord, au plus **3 captures par lot**, prises par la session
principale.

### TB0 — Planche cible (S, ≈ 1 $ fal.ai)
- Rapatrier 10 à 15 captures de ToB en session locale : campagne en vue large, moyenne et proche ;
  hiver ; colonie mineure ; capitale ; brouillard ; vue stratégique. Elles ne sont **pas
  versionnées** (droits tiers), comme pour l'annexe A de `campagne-vivante.md`.
- Repeindre 3 de nos captures (large, moyenne, proche) vers le rendu visé avec
  `fal-ai/nano-banana-2/edit`, en donnant nos images et la consigne de la bible. On obtient ainsi
  une **image cible par palier**, dérivée de notre propre carte, qui sert de critère de réussite
  aux lots suivants. Ces repeints peuvent être versionnés : ils viennent de nos captures.
- Validation du joueur sur la planche avant TB3.

### TB1 — Saisons visibles à tous les zooms (M, 0 $) — priorité 1
- Corriger le masquage par la carte satellite : appliquer un **delta saisonnier** (teinte et
  luminance du terroir, neige, chaume, dorure des blés) **par-dessus** la carte de couleur moyenne,
  au lieu de la remplacer. Variante si le delta ne suffit pas : précalculer 4 cartes de saison avec
  l'outil SS (`tools/`) et les fondre par `campaign_season`.
- Neige d'hiver lisible de loin : plaines du nord et de l'est blanchies par plaques. Corriger les
  taches au zoom moyen relevées dans CV1.
- Mer selon la saison : plus sombre et plus grise l'hiver, écume plus forte par tempête (le masque
  météo A existe déjà).
- Étalonnage par saison : printemps frais, été doré, automne chaud, hiver bleuté et désaturé. Les
  valeurs vont dans `data/ui/` ; on reprend le tableau de la bible § 12.6.
- Neige et suie sur les toits des villes 1:1 : brancher le uniform `snow` et la suie sur les
  maquettes ADR 0138.
- Critère : en vue large, on reconnaît la saison sans lire la date.

### TB2 — Désencombrement et lecture « à la ToB » (S, 0 $) — priorité 1
- Nuages et brumes : seulement en cas de météo réelle, plus fins en vue moyenne, jamais sur la
  province sélectionnée.
- Brouillard de guerre : un voile de parchemin léger plutôt qu'un délavage blanc, pour garder le
  relief lisible.
- Pictogrammes : un seul signe par ville et par palier ; les rosaces et étoiles ne s'affichent que
  dans un mode de carte.
- Étiquettes : hiérarchie typographique (capitale en petites capitales, villes en romain), pas de
  nom coupé en bord d'écran, et **noms de région** discrets en vue moyenne (aujourd'hui ils
  n'existent qu'en vue stratégique).
- Tracés : chemin d'armée et frontières FR1 moins saturés, réservés à la sélection.

### TB3 — Colonies et bâtiments qui poussent (L, ≈ 8-12 $ fal.ai) — priorité 2
- Rebrancher la croissance : remplacer le stub `replace_models` et `model_holder()` par une couche
  qui ajoute à la ville 1:1 des **quartiers de faubourg** selon la population simulée, et
  **l'enceinte** quand la fortification monte (murs, tours, porte, d'après le gabarit des villes
  emblématiques).
- Bâtiments hors les murs, comme ToB : ferme, moulin, vignoble, mine, saline, abbaye, marché,
  port. Chacun est placé sur le terroir de la province et a **3 niveaux** qui grossissent avec le
  bâtiment construit. La correspondance bâtiment → maquette et niveau va dans
  `data/map/building_models.json` (avec schéma), lue par le rendu ; le moteur expose déjà les
  bâtiments construits.
- Chantier visible : échafaudage, tas de pierres et grue à la place du « ⚒ ».
- Maquettes : chaîne image → 3D (ADR 0140 : `flux-2` → `bria` → `trellis`), environ 8 bâtiments
  × 3 niveaux ≈ 24 maquettes, à environ 0,10-0,35 $ pièce, reprises comprises. Usure procédurale
  (ADR 0136) ; règle « une seule main » de la bible.

### TB4 — Conséquences visibles de la guerre et des fléaux (S-M, 0 $) — priorité 2
- Suie et ruines sur les villes 1:1 saccagées ou prises d'assaut (même correctif que TB1).
- Peste : fosses, croix sur les portes, charrettes de morts et cloches, sans pictogramme.
- Champ de bataille marqué pendant quelques tours : tertre, corbeaux, débris.
- Siège : les engins en construction (N7, ADR 0128) apparaissent dans le camp au fil des tours.

### TB5 — Mer et côtes (M, ≈ 1 $ fal.ai) — priorité 3
- Falaises de craie et de granite, choisies par la pente et la géologie de la côte (Douvres, pays
  de Caux, Bretagne), plages et galets. Les textures sont faites avec fal.ai, comme pour HB.
- Différencier les mers : mer du Nord et Manche sombres, Atlantique houleux, Méditerranée claire ;
  ressac animé au trait de côte.

### TB6 — Lumière et atmosphère (S-M, 0 $) — priorité 3
- Essai SSIL, puis SDFGI si le budget d'images par seconde le permet (ADR 0123) ; mesurer avant
  d'adopter.
- Brouillard volumétrique bas pour la météo « brume du matin » (masque B), dans les vallées.
- Lumière dorée par défaut (soleil bas, ombres longues), comme sur les captures promotionnelles de
  ToB.

### TB7 — Interface de campagne (M, ≈ 2 $ fal.ai) — optionnel
- Panneau de province en bas d'écran, colonies côte à côte, bâtiments en grille d'icônes (écart G5
  de l'annexe A de `campagne-vivante.md`).
- Ornements enluminés des panneaux et de la vue stratégique (cartouches, rinceaux) faits avec
  fal.ai, dans le registre de la bible § 5.

## 4. Ordre et coût

| Vague | Lots | Coût fal.ai estimé |
|---|---|---|
| 1 | TB0, TB1, TB2 (TB1 et TB2 en parallèle, zones distinctes : shaders de terrain / couches de marqueurs) | ≈ 1 $ |
| 2 | TB3, TB4 | ≈ 8-12 $ |
| 3 | TB5, TB6 | ≈ 1 $ |
| option | TB7 | ≈ 2 $ |
| | **Total** | **≈ 12-16 $** (enveloppe proposée 25 $, ADR 0149) |

TB1 + TB2 donnent le plus grand gain visible pour 0 $. TB3 est le cœur de l'identité ToB.

## 5. Contraintes d'exécution

- Le conteneur cloud du 02/10 n'a ni Godot, ni Blender, ni accès à fal.ai, ni clé. Les lots de
  rendu et la planche TB0 se font en **session locale**, ou dans un environnement cloud avec ces
  outils et une politique réseau qui autorise fal.ai et les sites de référence.
- Au plus 6 agents par vague. La vérification visuelle reste à la session principale (CLAUDE.md).

## Sources

- PCGamesN, test PC de ToB : https://www.pcgamesn.com/total-war-thrones-of-britannia/thrones-of-britannia-pc-review
- PCGamesN, taille et détail de la carte : https://www.pcgamesn.com/a-total-war-saga-thrones-of-britannia/total-war-saga-thrones-of-britannia-map-size
- PC Gamer, carte 23 fois plus grande : https://www.pcgamer.com/total-war-saga-thrones-of-britannia-unveils-map-23-times-bigger-than-attilas/
- PC Invasion, colonies majeures et mineures : https://www.pcinvasion.com/total-war-saga-thrones-britannia-maps-revealed/
- The Sixth Axis, test : https://www.thesixthaxis.com/2018/04/30/total-war-saga-thrones-of-britannia-review/
- Steam, discussions sur l'art et les cartes d'unité : https://steamcommunity.com/app/712100/discussions/0/1796278072840773928/
- Total War Wiki, interface (vue stratégique Rome II / Attila) : https://totalwar.fandom.com/wiki/User_interface
- Archive.org, « Campaign Map First Look » (vidéo) : https://archive.org/details/TotalWarSagaThronesOfBritanniaCampaignMapFirstLook
