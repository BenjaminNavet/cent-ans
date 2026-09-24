# Audit A4 — assets libres (recherche web)

Auditeur A4, session de nuit 7, 2026-09-24. Recherche web uniquement (WebSearch/WebFetch) ;
**rien n'a été téléchargé dans le dépôt**. Objectif : identifier des assets gratuits et
redistribuables (CC0 de préférence, CC-BY acceptable avec crédit) pour enrichir la direction
artistique semi-réaliste du jeu (ADR `docs/decisions/0004-semi-realistic-art-direction.md`),
budget 0 $.

## État actuel (avant recherche)

Le dépôt n'a quasiment aucune dépendance à des assets tiers redistribués tels quels : voir
`CREDITS.md`.

- **Modèles 3D** (`game/assets/models/`) : 100 % générés procéduralement par des scripts Blender
  maison (low-poly). Aucun personnage humain riggé détaillé, aucun cheval, aucun arbre 3D.
- **Textures** (`game/assets/textures/`) : déjà CC0 Poly Haven (terrain : `aerial_grass_rock`,
  `aerial_mud_1`, `forrest_ground_01`, `aerial_rocks_01`, `sparse_grass`, `snow_field_aerial`,
  `aerial_beach_01` ; batailles : `bark_brown_02`, `castle_wall_varriation`, `roof_slates_02`,
  `clay_roof_tiles_02`, `thatch_roof_angled`, `plastered_wall_02`, `cobblestone_floor_01`,
  `wood_planks`). Aucune attribution requise (licence CC0).
- **Audio** (`game/assets/audio/`) : 100 % synthèse procédurale (numpy/scipy), aucun échantillon
  externe.
- **Icônes** (`game/assets/icons/`) : ~130 icônes game-icons.net (CC BY 3.0), déjà attribuées
  nommément dans `CREDITS.md`. Couverture jugée bonne, pas de lacune identifiée.
- **Héraldique** (`game/assets/heraldry/`) : écus dessinés procéduralement (Pillow) à partir des
  blasons de `data/factions/`.
- **Portraits** (`game/assets/portraits/`) : générés par IA (OpenRouter), dépenses dans
  `docs/budget.md`.
- **Polices** : polices système non redistribuées (Georgia/Palatino/Times) + Open Sans (OFL,
  fournie par Godot). Problème de portabilité identifié : sur une machine sans ces polices
  système (Linux notamment), le rendu « parchemin » retombe sur une police générique.

Les lacunes les plus nettes pour la direction artistique « Total War-like » : pas de figurines
humaines riggées avec animations, pas de chevaux, pas d'arbres 3D en vrai maillage, pas de ciel
HDRI pour les écrans fixes, pas de musique/bruitages échantillonnés, et une dépendance fragile
aux polices système.

## Top 15 à intégrer en priorité

Classement par rapport qualité/adéquation/facilité d'intégration, licence sûre en tête.

| # | Asset | Catégorie | Licence | Pourquoi en priorité |
|---|---|---|---|---|
| 1 | **Ultimate Modular Men Pack** (Quaternius) | Personnages | CC0 | Meilleur candidat pour remplacer/enrichir les figurines de bataille : modulaire, riggé, 24 animations, export GLB natif |
| 2 | **Ultimate Animated Animal Pack** (Quaternius, cheval + cheval blanc) | Chevaux | CC0 | Seule source de chevaux riggés/animés trouvée en CC0, compatible cavalerie avec #1 |
| 3 | **EB Garamond** (Google Fonts) | Police | OFL 1.1 | Corrige un vrai problème de portabilité (dépendance à Georgia/Palatino non redistribués) ; URL de téléchargement direct fiable |
| 4 | **IM Fell English** (Google Fonts) | Police | OFL 1.1 | Titres/manuscrit, complète EB Garamond pour le thème parchemin ; téléchargement direct |
| 5 | **Kenney — Fantasy UI Borders** | Ornements UI | CC0 | Cadres/bordures prêts à recolorer sépia pour matcher le style parchemin existant ; zip direct 350 Ko |
| 6 | **Quaternius — Medieval Village MegaKit** | Bâtiments | CC0 | 304 modèles modulaires, le plus complet pour un rendu rapproché des colonies |
| 7 | **Kenney — Castle Kit** | Bâtiments/props siège | CC0 | 75+ objets (murs, tours, portes, équipement de siège), URL zip directe vérifiée (2,2 Mo) |
| 8 | **Belfast Open Field** (Poly Haven HDRI) | Ciel HDRI | CC0 | Ciel couvert de plaine, idéal menu/cinématiques, URL directe `.hdr` |
| 9 | **Autumn Field Pure Sky** (Poly Haven HDRI) | Ciel HDRI | CC0 | Ciel dégagé « pur » réutilisable comme skybox neutre, URL directe |
| 10 | **Pine Tree 01 / Fir Tree 01** (Poly Haven) | Végétation | CC0 | Premiers vrais arbres 3D du projet (conifères, Flandre/Ardennes), URL FBX directe |
| 11 | **Grass Medium 01/02** (Poly Haven) | Végétation | CC0 | Touffes d'herbe géométriques en complément du plan-alpha procédural actuel, MultiMesh-friendly |
| 12 | **Parchment GUI** (zwonky, OpenGameArt) | Ornements UI | CC0 | Panneaux/boutons déjà « parchemin », cohérents avec le style actuel, poids négligeable |
| 13 | **« Church Bell.wav »** (Audeption, Freesound) | SFX | CC0 | Cloche de qualité pour la cloche de tour de jeu, remplacerait la synthèse procédurale |
| 14 | **« Swords Clash – High Quality #3 »** (Christopherderp, Freesound) | SFX | CC0 | Bruitage de combat net, complète `sword_clash.ogg` procédural |
| 15 | **« Lord of the Land » / « Village Consort »** (Kevin MacLeod, incompetech) | Musique | CC BY 4.0 | Meilleure source de musique réellement enregistrée trouvée, attribution simple et déjà rédigée |

Non retenus mais documentés ci-dessous par transparence : CMU Mocap (utile mais nécessite un
retargeting lourd), Mixamo (exclu, non redistribuable), bundle Sonniss GDC (exclu, licence
anti-redistribution explicite), blasons Wikimedia Commons « modernes » (très majoritairement
CC-BY-SA, donc exclus sauf deux exceptions CC0 listées en §8).

---

## 1. Personnages et soldats médiévaux 3D riggés + animations

| Asset | URL | Licence | Formats | Taille | Qualité | Téléchargement |
|---|---|---|---|---|---|---|
| **Ultimate Modular Men Pack** (Quaternius) | https://quaternius.com/packs/ultimatemodularcharacters.html · https://poly.pizza/bundle/Ultimate-Modular-Men-Pack-ZiH8muWqwQ | CC0 (Public Domain) | FBX, GLTF/GLB, OBJ, Blend | 11 personnages × 4 modules interchangeables (têtes/torses/jambes/armes), 24 animations chacun | 5/5 — modulaire, riggé, très large bibliothèque d'animations, export GLB directement exploitable par le pipeline Godot déjà en place | Téléchargement manuel requis : boutons JS « Download FBX »/« Download GLTF » sur poly.pizza, pas d'URL de fichier stable exposée côté serveur |
| **Animated Knight Pack** (Quaternius) | https://quaternius.com/packs/knightcharacter.html · https://quaternius.itch.io/lowpoly-animated-knight | CC0 1.0 | FBX, OBJ, Blend | 10 fichiers, non texturé | 4/5 — chevalier riggé complet (idle, death, jump, roll, run, walk, attack) mais un seul personnage, style 2018 | Manuel requis ; itch.io renvoie 403 aux requêtes automatisées |
| **RPG Character Pack** (Quaternius) | https://quaternius.com/packs/rpgcharacters.html | CC0 (cohérent avec le reste du catalogue, à reconfirmer sur la page) | FBX, OBJ, Blend | non indiquée | 3/5 — plus fantasy qu'historique, utile en complément (PNJ, seigneurs) | Manuel requis |
| **CMU Graphics Lab Motion Capture Database** | http://mocap.cs.cmu.edu/ (miroirs BVH via cgspeed) | Domaine public / gratuite (« peut être copiée, modifiée, redistribuée sans permission ; ne peut pas être revendue telle quelle ») | BVH, ASF/AMC, C3D | base complète : plusieurs Go ; sous-ensembles bien plus petits par sujet | 3/5 — bonne source d'animations brutes à retargeter (marche, course, combat) mais nécessite un travail de nettoyage/retargeting, pas de modèle associé | `curl -O http://mocap.cs.cmu.edu/subjects/<NN>/<NN>_<NN>.amc` (URLs stables par sujet/séquence) |
| **Mixamo** (Adobe) | https://www.mixamo.com | **Exclu — non redistribuable tel quel.** Les CGU Adobe autorisent l'usage commercial des animations *intégrées à un projet* mais interdisent explicitement de distribuer les fichiers bruts de personnages/animations à des tiers ou de constituer un pack d'assets ; un dépôt git public constitue une distribution à des tiers. | FBX | — | — | Ne pas committer. Usage local uniquement (rig/retarget hors dépôt), à valider juridiquement avant tout usage en production |

**Constat transverse** : aucune des sources Quaternius/Poly Pizza/itch.io n'expose d'URL de
téléchargement direct stable (boutons JavaScript, parfois redirection Discord pour les packs
« supporter ») ; seul le CMU Mocap a des URLs de fichiers stables. Un téléchargement manuel via
navigateur est nécessaire pour la quasi-totalité de cette catégorie.

## 2. Chevaux 3D riggés et animés

| Asset | URL | Licence | Formats | Taille | Qualité | Téléchargement |
|---|---|---|---|---|---|---|
| **Ultimate Animated Animal Pack** (Quaternius, contient « Horse » et « White Horse ») | https://quaternius.com/packs/ultimateanimatedanimals.html · https://poly.pizza/bundle/Animated-Animal-Pack-ILAPXeUYiS | CC0 1.0 | FBX, OBJ, Blend, glTF/GLB | 12 modèles au total (dont 2 chevaux), >12 animations chacun | 5/5 — meilleur candidat : walk/gallop/attack/death/kick/jump, directement compatible avec un cavalier de l'Ultimate Modular Men Pack | Manuel requis (bouton JS, pas d'URL de zip stable) |
| Farm Animal Pack (Quaternius) | https://quaternius.com/packs/farmanimal.html · https://poly.pizza/bundle/Farm-Animal-Pack-1kUvRTPLzT | CC0 | FBX, OBJ, Blend | 7 animaux | 2/5 — présence d'un cheval non confirmée, redondant avec l'Animated Animal Pack qui est plus riche | Manuel requis |
| Modèles individuels Horse/White Horse/Horse Statue (Quaternius via Poly Pizza) | https://poly.pizza/search/horse (filtrer auteur Quaternius) | CC0 | FBX, GLB | modèle unique | 3/5 — utile pour un cheval statique de décor (écurie), l'Animated Animal Pack couvre déjà mieux le besoin de cavalerie animée | Manuel requis |

## 3. Bâtiments médiévaux, remparts, et props (charrettes, tonneaux, palissades, tentes, siège)

| Asset | URL | Licence | Formats | Taille | Qualité | Téléchargement |
|---|---|---|---|---|---|---|
| **Kenney — Castle Kit** | https://kenney.nl/assets/castle-kit | CC0 1.0 (attribution non obligatoire, demandée par courtoisie) | OBJ/GLTF (ZIP), 75+ objets (murs, tours, portes, équipement de siège, personnages, drapeaux) | 2,2 Mo (`content-length` vérifié) | 4/5 — très bon point de départ modulaire pour fortifications basse-poly, à retexturer pour coller au style PBR | `curl -L -o kenney_castle-kit.zip "https://kenney.nl/media/pages/assets/castle-kit/a395102d20-1711543616/kenney_castle-kit.zip"` (miroir OpenGameArt : `curl -L -o kenney_castle-kit.zip "https://opengameart.org/sites/default/files/kenney_castle-kit.zip"`) |
| **Kenney — Tower Defense Kit** | https://kenney.nl/assets/tower-defense-kit | CC0 1.0 | ZIP, 160 fichiers, thème château | 5,4 Mo (vérifié) | 3/5 — variations utiles de tours/murs mais orienté tower-defense | `curl -L -o kenney_tower-defense-kit.zip "https://kenney.nl/media/pages/assets/tower-defense-kit/a402493eaa-1726471567/kenney_tower-defense-kit.zip"` |
| **Kenney — Medieval Town (Base)** | https://kenney.nl/assets/medieval-town-base | CC0 | OBJ+MTL (67×) + package Unity, 65 assets modulaires (murs, toits, sols, routes), sans texture | ~1,2 Mo (annoncé, non revérifié en HEAD) | 4/5 — bonne base modulaire pour un centre-ville à colombages, retexturage complet nécessaire | Manuel requis (bouton Download dynamique) |
| **Quaternius — Medieval Village Pack** | https://quaternius.com/packs/medievalvillage.html · https://poly.pizza/bundle/Medieval-Village-Pack-NsHhjhlrfY | CC0 | FBX, OBJ, Blend, GLTF/GLB, 39-44 modèles | non indiquée | 4/5 — style low-poly texturé cohérent, bon complément aux bâtiments procéduraux | Manuel requis |
| **Quaternius — Medieval Village MegaKit** | https://quaternius.com/packs/medievalvillagemegakit.html · https://quaternius.itch.io/medieval-village-megakit · https://store.godotengine.org/asset/quaternius/medieval-village-megakit/ | CC0 | FBX, OBJ, Blend, glTF, 304 modèles modulaires (murs int/ext, toits, escaliers) | non indiquée ; existe en versions Standard/Pro/Source | 5/5 — le plus complet, existe même en asset dédié sur le Godot Asset Store | Manuel requis (itch.io / Godot Asset Store) |
| **Quaternius — Modular Medieval Building Pack** | https://quaternius.com/packs/modularmedievalbuildings.html | CC0 | FBX, OBJ, Blend, 30 modèles | non indiquée | 3/5 — plus ancien (2017), moins riche que le MegaKit | Manuel requis |
| **Quaternius — Fantasy Props MegaKit** | https://quaternius.com/packs/fantasypropsmegakit.html | CC0 | FBX, OBJ, glTF, 200+ modèles (mobilier, outils, armes, étals, coffres) | non indiquée | 3/5 — utile pour props de marché/ville ; présence de tentes/tonneaux/charrettes spécifiques à vérifier après téléchargement | Manuel requis |
| Quaternius — LowPoly Medieval Weapons | https://quaternius.itch.io/lowpoly-medieval-weapons | CC0 | FBX/OBJ/Blend | non indiquée | 2/5 — armes individuelles, pertinent pour de l'équipement de personnage plutôt que des props de siège | Manuel requis |

**Engins de siège dédiés (trébuchet/beffroi/bélier)** : aucun pack CC0 spécifique trouvé sous ce
nom chez Quaternius ; les recherches sur OpenGameArt n'ont abouti qu'à des modèles payants
(CGTrader, RenderHub, Free3D), exclus. Le Castle Kit de Kenney (#1 ci-dessus) inclut de
l'« équipement de siège » dans son lot — à vérifier après extraction. Sinon, garder la
génération procédurale Blender actuelle (`siege_camp.glb`) pour ces pièces spécifiques.

Poly Haven n'a pas de collection « medieval » identifiée pour cette catégorie (catalogue dominé
par textures/HDRIs).

## 4. Végétation (arbres, herbe) et textures PBR de terrain complémentaires

Constat : Poly Haven ne propose **aucun chêne/hêtre feuillu européen classique** — son catalogue
« trees » (20 assets, vérifié en intégralité) est dominé par des conifères (« Pine Forest ») et
de la végétation côtière/sud-africaine hors sujet. ambientCG n'a pas de modèles d'arbres complets
(seulement souches/feuilles en textures). Un feuillu tempéré devra être cherché ailleurs (à
recouper avec les packs Quaternius de la catégorie 3, ou Sketchfab CC0, hors scope de cette
recherche). Formats Poly Haven : uniquement `.blend`/`.fbx` (pas de GLB natif — import FBX ou
reconversion via Blender nécessaire).

| Asset | URL | Licence | Formats/résolutions | Qualité | Téléchargement |
|---|---|---|---|---|---|
| Pine Tree 01 | https://polyhaven.com/a/pine_tree_01 | CC0 1.0 | blend/fbx, 1k/2k/4k/8k | 4/5 — bon conifère pour forêts du nord (Flandre, Ardennes) | `curl -O https://dl.polyhaven.org/file/ph-assets/Models/fbx/2k/pine_tree_01/pine_tree_01_2k.fbx` |
| Fir Tree 01 | https://polyhaven.com/a/fir_tree_01 | CC0 1.0 | blend/fbx, 1k/2k/4k/8k | 4/5 — variante sapin, diversifie le fond forestier | `curl -O https://dl.polyhaven.org/file/ph-assets/Models/fbx/2k/fir_tree_01/fir_tree_01_2k.fbx` |
| Fir Sapling Medium | https://polyhaven.com/a/fir_sapling_medium | CC0 1.0 | blend/fbx | 3/5 — jeune arbre, lisières de forêt | `curl -O https://dl.polyhaven.org/file/ph-assets/Models/fbx/1k/fir_sapling_medium/fir_sapling_medium_1k.fbx` |
| Pine Sapling Small | https://polyhaven.com/a/pine_sapling_small | CC0 1.0 | blend/fbx | 3/5 — remplissage de sous-bois | `curl -O https://dl.polyhaven.org/file/ph-assets/Models/fbx/1k/pine_sapling_small/pine_sapling_small_1k.fbx` |
| Tree Small 02 | https://polyhaven.com/a/tree_small_02 | CC0 1.0 | blend/fbx | 3/5 — à vérifier visuellement s'il passe pour feuillu tempéré | `curl -O https://dl.polyhaven.org/file/ph-assets/Models/fbx/2k/tree_small_02/tree_small_02_2k.fbx` |
| Dead Tree Trunk 02 | https://polyhaven.com/a/dead_tree_trunk_02 | CC0 1.0 | blend/fbx | 3/5 — prop décor forêt calcinée/champ de bataille | `curl -O https://dl.polyhaven.org/file/ph-assets/Models/fbx/1k/dead_tree_trunk_02/dead_tree_trunk_02_1k.fbx` |
| Tree Stump 01/02 | https://polyhaven.com/a/tree_stump_01 (et _02) | CC0 1.0 | blend/fbx | 3/5 — props de clairière/campement | `curl -O https://dl.polyhaven.org/file/ph-assets/Models/fbx/1k/tree_stump_01/tree_stump_01_1k.fbx` |
| **Grass Medium 01** | https://polyhaven.com/a/grass_medium_01 | CC0 1.0 | blend/fbx | 4/5 — touffe basse poly, candidate MultiMesh directe | `curl -O https://dl.polyhaven.org/file/ph-assets/Models/fbx/1k/grass_medium_01/grass_medium_01_1k.fbx` |
| **Grass Medium 02** | https://polyhaven.com/a/grass_medium_02 | CC0 1.0 | blend/fbx | 4/5 — variante pour alterner en MultiMesh | `curl -O https://dl.polyhaven.org/file/ph-assets/Models/fbx/1k/grass_medium_02/grass_medium_02_1k.fbx` |
| Moss 01 | https://polyhaven.com/a/moss_01 | CC0 1.0 | blend/fbx | 2/5 — couvre-sol ponctuel (pierres, ruines) | `curl -O https://dl.polyhaven.org/file/ph-assets/Models/fbx/1k/moss_01/moss_01_1k.fbx` |
| Fern 02 | https://polyhaven.com/a/fern_02 | CC0 1.0 | blend/fbx | 3/5 — sous-bois forestier | `curl -O https://dl.polyhaven.org/file/ph-assets/Models/fbx/1k/fern_02/fern_02_1k.fbx` |

Le projet a déjà `foliage_leaves.png`/`grass_clump.png` (textures alpha procédurales pour herbe en
MultiMesh) ; les modèles Grass Medium 01/02 sont un complément géométrique, pas un remplacement
indispensable.

## 5. Ciels HDRI (Poly Haven, CC0)

| Asset | URL | Licence | Résolutions/poids | Qualité | Téléchargement |
|---|---|---|---|---|---|
| **Belfast Open Field** | https://polyhaven.com/a/belfast_open_field | CC0 1.0 | 1k = 1,57 Mo, 2k = 6,20 Mo, 4k = 24,6 Mo, 8k = 97,1 Mo | 5/5 — ciel couvert plat, prairie rase, idéal campagne grise/hivernale | `curl -O https://dl.polyhaven.org/file/ph-assets/HDRIs/hdr/2k/belfast_open_field_2k.hdr` |
| **Autumn Field (Pure Sky)** | https://polyhaven.com/a/autumn_field_puresky | CC0 1.0 | 1k = 1,04 Mo, 2k = 4,17 Mo | 5/5 — ciel dégagé « pur » sans sol, idéal skybox neutre réutilisable | `curl -O https://dl.polyhaven.org/file/ph-assets/HDRIs/hdr/2k/autumn_field_puresky_2k.hdr` |
| Alps Field | https://polyhaven.com/a/alps_field | CC0 1.0 | 1k = 1,6 Mo, 2k = 6,5 Mo | 4/5 — partiellement nuageux, lumière franche, menu/campagne ensoleillée | `curl -O https://dl.polyhaven.org/file/ph-assets/HDRIs/hdr/2k/alps_field_2k.hdr` |
| Autumn Meadow | https://polyhaven.com/a/autumn_meadow | CC0 1.0 | 1k/2k/4k/8k (schéma identique) | 4/5 — prairie dégagée, alternative estivale | `curl -O https://dl.polyhaven.org/file/ph-assets/HDRIs/hdr/2k/autumn_meadow_2k.hdr` |
| Belfast Farmhouse | https://polyhaven.com/a/belfast_farmhouse | CC0 1.0 | 1k/2k/4k/8k (poids similaires à Belfast Open Field) | 3/5 — décor de ferme en périphérie, moins neutre comme skybox générique | `curl -O https://dl.polyhaven.org/file/ph-assets/HDRIs/hdr/2k/belfast_farmhouse_2k.hdr` (schéma d'URL non revérifié individuellement) |

Le jeu utilise déjà un `Sky` procédural Godot (ADR 0004) pour le rendu en jeu dynamique ; ces
HDRI sont surtout pertinents pour des fonds fixes (écrans de menu, cinématiques), pas pour
remplacer le ciel physique en temps réel.

## 6. Musique médiévale libre et effets sonores

Rappel de contexte : toute la musique et les SFX actuels (`game/assets/audio/`) sont 100 %
synthèse procédurale (numpy/scipy), aucun échantillon externe.

### Musique

| Morceau | URL | Licence | Format | Qualité | Notes |
|---|---|---|---|---|---|
| **« Lord of the Land »** (Kevin MacLeod) | https://incompetech.com/music/royalty-free/music.html (rechercher le titre) | **CC BY 4.0** — attribution requise : « Lord of the Land Kevin MacLeod (incompetech.com) — Licensed under Creative Commons: By Attribution 4.0 — https://creativecommons.org/licenses/by/4.0/ » | MP3 (téléchargement direct depuis la page) | 4/5 — ambiance de campagne, thème médiéval modéré | — |
| **« Village Consort »** (Kevin MacLeod) | https://incompetech.com/music/royalty-free/music.html (rechercher le titre) | CC BY 4.0, même formule d'attribution | MP3 | 4/5 — ambiance de village/cour | — |
| Catalogue incompetech (2000+ morceaux) | https://incompetech.com/music/royalty-free/?keywords=medieval · liste complète : https://incompetech.com/music/royalty-free/full_list.php | CC BY 4.0 (uniforme sur tout le catalogue) | MP3 | 3-4/5 selon morceau | Filtrage par mots-clés « medieval »/« battle »/« royal » à faire manuellement |
| Musopen — période médiévale | https://musopen.org/music/period/medieval/ | Licence libre par mission (Musopen collecte spécifiquement des interprétations libres de droits) | MP3 gratuit, FLAC pour donateurs | 4/5 mais catalogue **surtout des partitions**, peu d'enregistrements audio médiévaux réels | Vérifier la disponibilité audio avant de compter dessus |
| Collections archive.org (« Music of Medieval France, 1200-1400 », « Music From The Middle Ages and The Renaissance ») | https://archive.org/details/music-of-medieval-france-1200-1400-sacred-and-secular-nikolaus-harnoncourt-dvg · https://archive.org/details/Music-of-Middleages_and_Renaissance | **Statut incertain** — probablement des rips d'enregistrements commerciaux sans droit clair | — | — | **À écarter ou vérifier item par item** ; ne pas assumer domaine public du seul fait d'être hébergé sur archive.org |

**Verdict** : incompetech (Kevin MacLeod, CC BY 4.0) est la source la plus sûre et directement
exploitable, licence et formule d'attribution vérifiées texte en main.

### Effets sonores

| Son | URL | Licence | Format | Qualité | Téléchargement |
|---|---|---|---|---|---|
| **« Sword Slash Attack »** (qubodup, Freesound) | https://freesound.org/people/qubodup/sounds/184422/ | CC BY 3.0 (attribution requise) | FLAC, 4,7 s, 226,6 Ko | 4/5 | Manuel (connexion requise) |
| **« Swords Clash – High Quality #3 »** (Christopherderp, Freesound) | https://freesound.org/people/Christopherderp/sounds/364531/ | **CC0** | WAV, 2,1 s, 360,1 Ko, 44,1 kHz/16 bit stéréo | 4/5 | Manuel (connexion requise) |
| **« Church Bell.wav »** (Audeption, Freesound) | https://freesound.org/people/Audeption/sounds/425172/ | **CC0** | WAV, 31,6 s, 5,3 Mo, 44,1 kHz/16 bit stéréo | 5/5 — excellent pour la cloche de tour | Manuel |
| « Medieval bells of doom » (Toivo161, Freesound) | https://freesound.org/people/Toivo161/sounds/266007/ | Non confirmée — à vérifier avant usage | — | — | Manuel |
| « Running Horse with a Rider » (sonically_sound, Freesound) | https://freesound.org/people/sonically_sound/sounds/645465/ | **CC-BY-NC — EXCLU** (usage commercial interdit) | — | — | Rechercher une alternative CC0/CC-BY via `https://freesound.org/search/?q=horse+gallop&f=license%3A%22Creative+Commons+0%22`, non vérifiée individuellement |
| « Crowd Cheering » (SoundsExciting / SoundBiterSFX, Freesound) | https://freesound.org/people/SoundsExciting/sounds/365132/ · https://freesound.org/people/SoundBiterSFX/sounds/730908/ | Non confirmée — à vérifier page par page | — | — | Manuel |
| Cor de guerre, tambour de marche, fanfare, feu, canon/bombarde, ambiance de camp | https://freesound.org/browse/tags/medieval/ | Mixte (CC0/CC-BY/CC-BY-NC selon le fichier) | — | — | **Chaque son doit être vérifié individuellement** ; un faux positif CC-BY-NC a été trouvé parmi des résultats qui semblaient CC0 à première vue |
| Bundle Sonniss #GameAudioGDC | https://sonniss.com/gdc-bundle-license/ | **Exclu, vérifié** : usage commercial/personnel autorisé sans attribution MAIS *« Licensee may not distribute, publish, sub-license or otherwise supply the sound effects as sound effects to any other person »*, y compris en asset pack ; entraînement d'IA explicitement interdit | — | — | Ne pas committer les fichiers dans le dépôt (cloner le dépôt reviendrait à « fournir les effets sonores » à un tiers) |

**Freesound — point d'attention technique** : aucun téléchargement scriptable sans authentification
OAuth2 (`curl -H "Authorization: Bearer {token}" 'https://freesound.org/apiv2/sounds/<id>/download/'`
nécessite un jeton obtenu via leur flux d'authentification) ; les previews basse qualité sans
token ne sont pas destinées à la redistribution. Téléchargement manuel requis pour tous les sons
Freesound listés ci-dessus.

## 7. Polices médiévales libres, ornements UI, icônes

### Polices (OFL 1.1)

Toutes vérifiées dans le dépôt officiel `google/fonts/ofl/` (licence OFL 1.1, `OFL.txt` présent
dans chaque dossier, redistribution explicitement autorisée).

| Police | Usage suggéré | Format | Taille | Qualité | Téléchargement |
|---|---|---|---|---|---|
| **EB Garamond** | Corps de texte parchemin (remplace Georgia/Palatino) | TTF variable (`wght`) | ~851 Ko (roman) + 754 Ko (italique) | 5/5 — Garamond révival très lisible, look manuscrit élégant | `curl -LO https://raw.githubusercontent.com/google/fonts/main/ofl/ebgaramond/EBGaramond%5Bwght%5D.ttf` + `curl -LO https://raw.githubusercontent.com/google/fonts/main/ofl/ebgaramond/OFL.txt` |
| **IM Fell English** | Titres, manuscrits, citations, codex | TTF (roman + italique) | 195 Ko + 202 Ko | 5/5 — fac-similé de fonderie anglaise du XVIIe, texture « imprimé ancien » adaptée au thème | `curl -LO https://raw.githubusercontent.com/google/fonts/main/ofl/imfellenglish/IMFeENrm28P.ttf` (+ `IMFeENit28P.ttf`) |
| Cinzel | Titres de factions, écrans de menu (capitales gravées) | TTF variable (`wght`) | 125 Ko | 4/5 — inspiré d'inscriptions romaines, un peu « antique » plutôt que gothique | `curl -LO https://raw.githubusercontent.com/google/fonts/main/ofl/cinzel/Cinzel%5Bwght%5D.ttf` |
| MedievalSharp | Gros titres d'ambiance forte, usage ponctuel | TTF | 149 Ko | 3/5 — très marqué « fantasy », illisible en corps de texte long | `curl -LO https://raw.githubusercontent.com/google/fonts/main/ofl/medievalsharp/MedievalSharp.ttf` |
| UnifrakturMaguntia (optionnelle) | Lettrines/gothique allemand, Empire | TTF | 89 Ko | 3/5 — belle fraktur mais lisibilité faible en français | `curl -LO https://raw.githubusercontent.com/google/fonts/main/ofl/unifrakturmaguntia/UnifrakturMaguntia-Book.ttf` |
| Pirata One (optionnelle) | Alternative gothique plus lisible que Fraktur | TTF | 56 Ko | 3/5 | `curl -LO https://raw.githubusercontent.com/google/fonts/main/ofl/pirataone/PirataOne-Regular.ttf` |
| Almendra (optionnelle) | Corps de texte alternatif, 4 graisses | TTF ×4 | 35-69 Ko chacune | 3/5 — correct mais moins qualitatif qu'EB Garamond | `curl -LO https://raw.githubusercontent.com/google/fonts/main/ofl/almendra/Almendra-Regular.ttf` |

**Recommandation** : EB Garamond (corps) + IM Fell English (titres) suffisent à éliminer la
dépendance aux polices système, avec Cinzel en option pour les écrans de menu. Copier chaque
`OFL.txt` à côté des fichiers et lister l'attribution des designers d'origine dans `CREDITS.md`.

### Ornements / cadres UI (CC0)

| Asset | URL | Licence | Format | Taille | Qualité | Téléchargement |
|---|---|---|---|---|---|---|
| **Kenney — Fantasy UI Borders** | https://kenney.nl/assets/fantasy-ui-borders | CC0 | PNG (140 sprites) + tilesheet | 350 Ko (zip, vérifié) | 5/5 — cadres/bordures génériques de bonne qualité, faciles à recolorer sépia | `curl -LO https://kenney.nl/media/pages/assets/fantasy-ui-borders/ab29cd0165-1701602367/kenney_fantasy-ui-borders.zip` |
| **Parchment GUI** (zwonky, OpenGameArt) | https://opengameart.org/content/parchment-gui | CC0 | 4× PNG (boutons, panneaux, étiquettes, emplacements) | ~46 Ko au total | 4/5 — déjà « parchemin » natif, très proche du style actuel, mais basse résolution | `curl -LO https://opengameart.org/sites/default/files/panels.png` (+ `buttons_14.png`, `labels_0.png`, `slots.png`) |
| FANTASY-parchment-set (Melissa Krautheim, OpenGameArt) | https://opengameart.org/content/fantasy-parchment-set | CC0 | PNG (icônes de parchemin, lettres, cartes) | quelques Ko/fichier | 3/5 — complément ponctuel (icône « lettre », « carte roulée ») | Via page OpenGameArt |

### Icônes

Le projet utilise déjà ~130 icônes game-icons.net (CC BY 3.0, cf. `CREDITS.md`) couvrant
bâtiments, unités, ressources, médecine, commerce. **Couverture jugée déjà bonne** — aucune
lacune évidente identifiée. Pour tout besoin ponctuel futur, rester sur game-icons.net (même
pipeline de recoloration déjà en place) plutôt que d'introduire une nouvelle source stylistique.

## 8. Cartes et héraldique

**Point d'attention majeur** : la quasi-totalité des blasons SVG « modernes » sur Wikimedia
Commons — notamment tout le corpus très visible de l'utilisateur Sodacan (armes royales de
France, d'Angleterre 1340-1367, etc.) — est en **CC-BY-SA**, donc **exclue** par le mandat du
projet. Même chez un même auteur (ex. Tom-L, vectorisations d'après l'Armorial de Gelre), la
licence varie fichier par fichier : vérification individuelle obligatoire.

| Blason | URL | Licence (vérifiée) | Format/taille | Qualité | Téléchargement |
|---|---|---|---|---|---|
| **Armoiries du comte de Flandre** (d'après l'Armorial de Gelre, folio 80r, avant 1396) | https://commons.wikimedia.org/wiki/File:Coat_of_Arms_of_the_Count_of_Flanders_(according_to_the_Gelre_Armorial).svg | **CC0 1.0** | SVG, 283×732, 35 Ko | 5/5 — source d'époque exacte (avant 1396), pertinent pour la faction Flandre | `curl -L -o flanders.svg "https://commons.wikimedia.org/wiki/Special:FilePath/Coat%20of%20Arms%20of%20the%20Count%20of%20Flanders%20(according%20to%20the%20Gelre%20Armorial).svg"` |
| **Lion rampant de sable** (meuble héraldique isolé, Armorial de Gelre, avant 1396) | https://commons.wikimedia.org/wiki/File:Lion_Rampant_sable_(Gelre).svg | **CC0 1.0** | SVG, 226×261, 16 Ko | 4/5 — meuble générique réutilisable (Flandre, Brabant, etc.) | `curl -L -o lion_sable.svg "https://commons.wikimedia.org/wiki/Special:FilePath/Lion%20Rampant%20sable%20(Gelre).svg"` |

Fichiers explorés mais **écartés** (CC-BY-SA confirmée) : `Royal Arms of England (1340-1367).svg`
(Sodacan, CC-BY-SA + GFDL), `Coat of Arms of Kingdom of France.svg`, `Arms of the Kingdom of
France.svg`, `Royal Coat of Arms of France.svg` (toutes CC-BY-SA), `Blason maison
Plantagenêt.svg` (Thom.Lanaud, CC-BY-SA 4.0), `Coat of Arms of Robert de Cassel (according to
the Gelre Armorial).svg` (Tom-L, CC-BY-SA 3.0 malgré la même série que le fichier Flandre CC0).

**Recommandation** : le projet dessine déjà ses propres écus par Pillow à partir de
`data/factions/`, donc pas de besoin urgent d'intégrer des armoiries Wikimedia dans le jeu. Les
deux fichiers CC0 ci-dessus peuvent servir de **référence visuelle** fidèle à l'Armorial de Gelre
pour calibrer le générateur procédural, sans être intégrés tels quels (évite tout mélange de
licences dans `game/assets/heraldry/`). Pour toute armoirie supplémentaire, filtrer
systématiquement par licence CC0/PD sur la page du fichier — l'appartenance à la catégorie
« Armorial de Gelre » ne garantit pas à elle seule une licence CC0.

---

## Exclusions et points de vigilance (résumé)

- **Mixamo** : CGU Adobe interdisent la distribution des FBX bruts à des tiers ; un dépôt git
  public constitue une distribution. Exclu, sauf usage strictement local hors dépôt.
- **Bundle Sonniss #GameAudioGDC** : licence vérifiée interdisant explicitement de « fournir les
  effets sonores en tant qu'effets sonores » à un tiers, y compris en asset pack. Exclu.
- **Blasons Wikimedia « modernes »** (Sodacan et la majorité du corpus) : CC-BY-SA, exclus. Seuls
  deux fichiers CC0 issus de l'Armorial de Gelre ont été retenus, à vérifier individuellement pour
  tout ajout futur.
- **Collections de musique archive.org** : statut de droit d'auteur souvent flou (rips de disques
  commerciaux) ; ne pas assumer domaine public du seul fait de l'hébergement sur archive.org.
- **Freesound** : licence non uniforme même au sein d'une recherche par mots-clés (un faux
  positif CC-BY-NC trouvé sur une recherche « horse gallop » qui semblait CC0) ; vérifier chaque
  son individuellement avant usage. Téléchargement nécessite une authentification (pas de `curl`
  anonyme).
- **Formats Poly Haven pour les modèles 3D** : uniquement `.blend`/`.fbx`, pas de GLB natif —
  prévoir une étape de conversion (Blender ou `tools/blender_scripts/`) avant import dans Godot.
- **Quaternius/Poly Pizza/itch.io** : aucune URL de téléchargement direct stable trouvée pour les
  packs de personnages/animaux/bâtiments (boutons JavaScript, itch.io bloque les requêtes
  automatisées) — téléchargement manuel via navigateur nécessaire pour la quasi-totalité des
  catégories 1, 2 et 3.
