# Audit visuel de la carte de campagne (29/10/2026)

Auteur : directeur artistique (agent d'audit), sur `main` f913f4f9b. Rien n'a été modifié dans le code ni dans les données.
Captures et dumps : `out/audit/visuel/` (non commités). Planches de deux vues à 480 px dans `out/audit/visuel/sheets/`.
Pleine résolution (1280×800) dans `out/audit/visuel/shots/`. Dumps JSON du même instant dans `out/audit/visuel/dumps/`, mesures
couleur dans `out/audit/visuel/stats.py`.
Protocole : `client/run_bg.sh --offscreen` muet, graine 1337, France, `CENT_ANS_CAMERA` en variable d'environnement, 900 images
d'attente. `regional_ground.bound = true` est vérifié dans **chaque** dump avant de juger le sol.
La machine était très chargée (charge 7 à 25, 3 emplacements `heavy.sh` pris en permanence par d'autres lots). Les temps d'image des
dumps (22 à 44 ms) ne valent donc **pas** mesure de fps. Tous les coûts fps ci-dessous sont des estimations à confirmer au banc.

Mesure couleur, faite sur la bande centrale du monde (hors HUD) : teinte, saturation et valeur moyennes (HSV). La cible de la bible
de palette (`docs/design/palette.md`) pour l'herbe de printemps à midi est : teinte 82-106°, saturation 0,22-0,42, valeur 0,36-0,56.

| Vue | Météo | Teinte | Sat. | Val. | Lecture |
|---|---|---|---|---|---|
| Paris d 600 | pluie | 73 | 0,19 | 0,45 | gris-olive |
| Paris d 200 | pluie | 77 | 0,17 | 0,41 | gris-vert boueux |
| Paris d 60 | pluie | 100 | 0,18 | 0,35 | gris-bleu, sombre |
| Paris d 25 | pluie | 107 | 0,19 | **0,30** | trop sombre |
| **Paris d 60** | **clair** | 100 | **0,17** | **0,36** | même gris-vert sans pluie : la pluie n'est pas la cause |
| Mont-Saint-Michel d 40 | clair | 143 | 0,19 | 0,46 | baie en nappe cyan-blanc |
| Loire d 80 | clair | 94 | **0,16** | 0,37 | terne |
| Loire d 80, automne | clair | 37 | 0,20 | **0,34** | brun uniforme |
| Castille d 400 | clair | 96 | **0,13** | 0,46 | même vert gris que la Normandie |
| Alpes d 120 | clair | 185 | **0,08** | 0,53 | plâtre rose-gris |
| Paris d 200, hiver | — | 212 | **0,04** | 0,57 | drap blanc-gris |

Aucune vue de campagne n'atteint la saturation cible. La campagne au plus près est 25 % plus sombre que la borne basse.

---

## (a) Verdict global

**Note globale : 4,8 / 10.**

Le squelette est solide :
- le relief exagéré des Alpes et de l'Écosse a de vraies formes ;
- Paris en maquette Meshy est un bel objet ;
- les plaques de villes sont nettes ;
- la vue parchemin a une identité.

Mais la carte 3D ne ressemble à **rien de réel** : ni à une vue aérienne, ni à une carte peinte. C'est un sol **gris-vert sans
lumière**, à 17 m par pixel au plus près, où les cultures se lisent en facettes floues gris-bleu. Les villages sont des cubes gris
épars. La mer est une dalle grise cerclée de cyan. Les montagnes sont en plâtre rose sans neige ni étages. Il n'y a **pas de ciel**,
et un lavis politique rouge efface des régions entières (la Guyenne).

Le joueur voit d'abord la météo de Paris, souvent la pluie : le premier écran est gris sur gris.

Les correctifs successifs (TS, TERRAIN, TERRAIN2, LUM, EAU, FORET) ont chacun **retiré** un défaut : damier, noirs, vermicelles,
boules de coton. Aucun n'a posé une **image cible positive**. Le résultat est une moyenne prudente, désaturée et brumeuse, où rien ne
domine. Le mot d'ordre de l'ADR 0710 (« la lisibilité vient du contraste de valeurs et de l'atmosphère, pas de la saturation ») a
été appliqué à moitié : la saturation a baissé, mais la valeur n'a jamais été hiérarchisée (forêt sombre / champ clair / eau / ville
chaude). L'atmosphère, elle, est un voile uniforme.

| Sous-système | Note | En une phrase |
|---|---|---|
| Zooms, caméra, transitions | 5 | Bascule sèche 3D → parchemin à 1200 ; caméra à 30° qui ne montre jamais l'horizon ; min. 22 u ≈ 16 km au-dessus du sol. |
| Sol : parcelles, terroirs, textures, transitions | 3,5 | La carte de couleur à 360 m/texel pèse encore 45 % au plus près ; facettes de Voronoi gris-bleu ; aucune différence Castille / Normandie. |
| Relief, montagnes | 5,5 | Formes réussies (massifs), matière nulle : pas d'étagement, neige grise invisible, pas de forêt de pente. |
| Côtes, mer | 4 | Mer gris ardoise plate, liseré cyan néon, pas de gradient de profondeur lisible, brume qui s'arrête au rivage. |
| Fleuves | 4,5 | Fils sombres de 2 à 4 px, sans berge ni ripisylve ; la Seine n'existe pas en vue moyenne ; Gironde en coin cyan. |
| Forêts, végétation | 5 | Massifs baltes corrects ; en plaine, « grains de poivre » d'arbres isolés et taches vert-jaune plus claires que les champs. |
| Villes, villages, châteaux, abbayes | 5,5 | Paris et les 8 villes emblématiques sont bonnes ; les 3 900 lieux génériques sont des cubes gris (TRELLIS) qui se lisent comme des gravats. |
| Routes et champs | 3,5 | Routes en traits blancs rectilignes « rail » ; champs sans géométrie médiévale (ni lanières, ni soles, ni bocage). |
| Armées et marqueurs | 6 | Figurines animées et étendard lisibles de près ; de loin, plaque chiffrée seule. |
| Étiquettes, plaques, frontières, couleurs politiques | 5,5 | Plaques excellentes ; lavis de position (rouge 0,26) qui noie le terrain ; frontières de parchemin multicolores criardes. |
| Lumière, ciel, brume, atmosphère, saisons, météo, nuit | 3 | Fond gris uni à la place du ciel, ambiance bleue trop forte, brouillard en voile blanc qui épargne la mer, hiver en drap, automne en boue, aucune nuit. |
| Palette, unité de style | 4 | Trois mondes : parchemin crème, 3D gris-bleu, bataille jaune-vert. |
| Échelle, densité | 4,5 | Maquettes de 3,6 à 12,6 km : Saint-Denis est « dans » Paris ; trop de villages gris en vue moyenne. |
| Pop-in, LOD | 5,5 | Peu de pop-in mesuré ; mais le sol change de nature trois fois (carte de couleur → texture → parchemin). |
| Lisibilité stratégique | 5,5 | On lit qui possède quoi (plaques, couleurs) ; on ne lit ni le terrain (forêt, marais, col), ni les routes, ni les rivières qui comptent pour le mouvement (ADR 0343). |
| Cohérence avec le parchemin et la bataille | 4,5 | La campagne est le maillon gris entre une UI chaude et une bataille ensoleillée. |

---

## (b) Diagnostic par gravité

La planche citée est entre crochets. Le fichier pleine résolution porte le même nom de vue dans `shots/`.

### Critique (G1) : ce que le joueur voit d'abord et partout

**G1-1. Pas de ciel, pas d'horizon, un fond gris uni.**
Preuves : [s2 `v05_paris200` / `v06_paris60`], [`v07_paris25_640.png`], [s3 Alpes], [s8 `c1_paris60_clear`] (même mur gris
par temps clair).
- Ce qu'on voit : le haut de chaque vue basse finit dans un mur gris-crème. Bande haute mesurée de 126 à 185 sur 255 ; aucune teinte
  de ciel.
- Cause, caméra : la caméra de campagne efface en `Color::srgb(0.74, 0.76, 0.76)` (`client/src/campaign/camera/mod.rs:286`).
  Il n'existe ni ciel ni horizon pour la campagne (`data/fx/horizon.json` ne sert qu'à la bataille).
- Cause, inclinaison : elle varie linéairement de 30° (d 22) à 70° (d 2600) (`camera/rig.rs:184-186`). Avec un champ de 55°, le
  rayon du haut de l'écran plonge encore de 2,5° à d 22 : l'horizon n'entre jamais dans le cadre, et la brume
  (`weather/atmosphere.rs`) habille le vide.
- Nature : décision de design, non prise. C'est le défaut qui sépare le plus la carte des références du genre (horizon courbe et
  ciel en vue basse).

**G1-2. Lumière froide et plate : ambiance bleue trop forte, soleil bas et faible, valeur trop basse.**
Preuves : tableau de mesures ; [s2] ; [`v07_paris25_640.png`] (sol gris-bleu) ; [s8 `c1_paris60_clear`], **par temps clair** :
saturation 0,17, valeur 0,36. La pluie du premier tour aggrave l'image, mais elle n'en est pas la cause.
- Cause, réglages : `data/fx/campaign_lighting.json` met une ambiance `color [0.64, 0.72, 0.9]`, `energy 0.56` (avec
  `sky_contribution 0.2`) contre un soleil `sun_energy 1.5` à 20° (`atmosphere.json` `campaign.seasons.spring`).
- Rapport soleil / ciel : l'éclairement solaire au sol vaut 1,5 × sin 20° ≈ 0,51, contre ≈ 0,56 pour le ciel bleu. Le ciel
  domine : les ombres et les plats se teintent de bleu, et le relief ne se lit que par l'ombre cuite.
- Comparaison : la bataille est passée à un rapport soleil / ciel réaliste (ADR 0406, 0755). La campagne non.
- Le soleil « fin d'après-midi » perpétuel (bornes 15-28°) est un choix de lecture du relief. Il coûte toute la lumière de midi
  que la bible de palette prend pour référence.
- Nature : réglage.

**G1-3. Le sol au plus près est une bouillie à 360 m/texel mêlée de facettes de Voronoi.**
Preuves : [`v07_paris25_640.png`] (facettes gris-bleu à bord flou), [s2 `v06_paris60`], [s7 Loire automne].
- Cause 1, la carte de couleur : dans `terrain.wesl::regional_ground` (l. 2404-2408),
  `painted = mix(g.color, tinted, mix(0.45, 1.0, far_w))`. La carte de couleur cuite (`colormap.bc1`, 2 texels par unité, soit
  ≈ 360 m/texel) garde **45 % du poids au plus près**, et 100 % dès d ≈ 500 (`far_w = smoothstep(0.05, 0.5, footprint)`).
- Cause 2, l'échelle des textures : au plus près, un pixel vaut ≈ 17 m. Les textures régionales (tuile de 25 à 120 m) et le micro
  (tuile de 10 m) passent sous 2 à 7 pixels par tuile. Elles sont moyennées par le mip et n'apportent rien.
  - Le micro s'efface entre 0,004 et 0,02 d'empreinte (`ground.rs`) : **jamais visible** à d ≥ 22.
  - Le désordre d'herbe (`clutter`) ne s'affiche que sous d 2,6, et le plancher de la caméra est 20.
- Cause 3, les parcelles : les facettes de Voronoi (ADR 0213) restent des polygones isotropes de 260 à 300 m. Ce ne sont ni des
  lanières d'openfield, ni du bocage. L'ADR 0787 a fondu leurs bords et réduit leur contraste au lieu de leur donner une forme.
  Résultat : une « peau de camouflage ».
- Nature : technique et design. À 17 m/px, la carte est un **rendu de photo satellite**. Il faut penser le sol comme une image
  Sentinel-2 / Landsat (parcellaire, haies, noyaux villageois), pas comme un splat de textures de près.

**G1-4. Le lavis politique efface le terrain des voisins.**
Preuves : [s4 `v08_guyenne300`] (toute la Guyenne en brique sombre, plus aucune lecture du Bordelais ni des Landes), [s6
`v14_calais250`] (Kent rouge).
- Cause : `data/map/stance_fill.json` donne `alpha.enemy 0.26` et `self 0.34`, plein effet au-delà de d 260 (`zoom.far_distance`).
  Le mode `political` est actif par défaut (`modes.political 1.0`). Le lavis se pose sur un sol déjà sombre et désaturé : il le
  remplace au lieu de le teinter.
- Contexte : ADR 0175 et 0155 (rouge réservé à l'ennemi).
- Nature : réglage et design. La position diplomatique doit vivre dans la **frontière** (bande intérieure) et dans les plaques, pas
  dans un aplat sur tout le territoire, en vue 3D.

### Majeur (G2)

**G2-1. Montagnes en plâtre : pas d'étages, neige invisible.**
Preuves : [s3 `v09_alps400` / `v10_alps120`] (saturation 0,08, teinte 141-185°).
- Ce qu'on voit : au printemps, la ligne de neige est à 2 000 m (`campaign_seasons.json` `snow.line_m[0]`). Le massif du Mont-Blanc
  devrait être blanc au-dessus. On voit une roche rose-mauve uniforme et des fonds de vallée blanchis par la brume.
- Cause 1 : la neige est volontairement « jamais blanc pur » (ADR 0710, 0750). `snow.plain_color` vaut `[0.70, 0.68, 0.64]` (gris
  chaud) et `shade_color` `[0.55, 0.57, 0.63]`. Après l'étalonnage et la brume chaude, la neige a **la même valeur que la roche**.
- Cause 2 : aucun étagement par altitude réelle : prairie de fond, hêtraie-sapinière, mélézin, alpage, éboulis, roche, neige.
  La carte de couleur bascule en roche dès 1 700 m (`colormap_style.yaml` `rock_from_m`) et n'a pas de forêt de pente.
- Cause 3 : la brume de vallée (`campaign_post.json` `valley_mist`) remplit les vallées de blanc par temps clair.
- Nature : réglage, plus une passe d'étagement dans `terrain.wesl` et la carte de couleur.

**G2-2. Mer en dalle grise et liseré néon.**
Preuves : [s5 `v13_baltic400`] (Frisches Haff cyan-gris uniforme, bord cyan vif), [s6 `v14_calais250`] (la Manche ressemble à un
fleuve gelé), [s1 `v03_nfr1000`], [s8 `c4_montstmichel40_clear`] : la baie du Mont-Saint-Michel est une nappe cyan-blanc plate à liseré blanc
dur, comme une feuille de plastique. On n'y lit ni estran, ni chenaux, ni le Mont.
- Cause 1, la profondeur : elle est estimée depuis la distance à la côte, à 4,5 m par pixel (ADR 0750), donc sans vraie
  bathymétrie. Le large prend partout la même couleur ardoise, et la bande peu profonde fait une quinzaine de pixels carte.
- Cause 2, le liseré : `sea_life.json` `turquoise_saturation 0.62` sur ce fond gris donne un liseré lumineux.
- Cause 3, l'horizon : sans ciel (G1-1), la mer ne reflète rien et reste plate.
- Nature : données (bathymétrie absente) et réglage.

**G2-3. Le brouillard s'arrête au rivage.**
Preuve : [s7 `v17_normandy200_fog`].
- Ce qu'on voit : la terre est noyée dans un voile blanc opaque, mais la mer reste une dalle gris sombre à bord net. Le brouillard
  ne se lit pas comme de l'air, il se lit comme une peinture blanche sur la terre.
- Cause : la brume du sol (`terrain.wesl`, nappes `morning_mist` et météo `weather_ground`) est peinte dans le fragment du sol. Le
  shader de mer (`water/shaders/sea.wesl`) ne reçoit que le `DistanceFog` de la caméra.
- Nature : technique. La brume doit être une seule passe sur la profondeur : `post.wesl` existe déjà (ADR 0710).

**G2-4. Saisons caricaturales.**
- Hiver [s6 `v15_paris200_winter`] : Bassin parisien entièrement blanc-gris (saturation 0,04). Forêts en taches grises, toute
  lecture perdue. `snow.plain_share 0.93` couvre la plaine à 93 % à chaque tour d'hiver. Historiquement, le manteau neigeux de
  plaine en Île-de-France est épisodique. La lecture attendue d'un hiver vu d'avion : labours bruns, prés vert-olive pâle, forêts
  brun-violet, givre et neige par plaques.
- Automne [s7 `v16_loire80_autumn`] : tout le sol vire au brun-taupe (teinte 37°, valeur 0,34). Ni chaumes dorés, ni prés verts,
  ni vignes rouillées. Le Val de Loire, région de vigne, ne montre aucune vigne.
- Cause : les teintes de saison multiplient l'ensemble du sol (`season_k`, `SG_SEASON_STRENGTH`), au lieu de changer les classes
  de culture une à une.
- Nature : réglage et design.

**G2-5. Villages génériques en cubes gris, trop nombreux en vue moyenne.**
Preuves : [s6 `v14_calais250`], [s4 `v08_guyenne300`], [s7 `v17_normandy200_fog`].
- Cause 1, les modèles : un seul glb TRELLIS par type et famille (`data/art/dn_campaign_models.json`, `village_west`,
  `town_west`…). Texture BC1 assombrie, pas de toits de couleur lisibles.
- Cause 2, la densité : maquette de village de 5 u (3,6 km) visible jusqu'à 900 (`data/art/town_maquettes.json`), soit 3 900
  lieux et hameaux indexés.
- Cause 3, l'ancrage : aucun noyau au sol (jardins, vergers, halo de terre battue) n'ancre la maquette. Elle se lit comme un
  rocher.
- Contraste : les villes emblématiques Meshy (Paris [`v07_paris25_640.png`]) sont d'un autre niveau.
- Nature : asset, plus réglage de densité.

**G2-6. Échelle des maquettes contre géographie.**
Preuve : [`v07_paris25_640.png`] : la plaque « Saint-Denis » flotte sur la cathédrale.
- Cause : Paris fait 17,5 u ≈ 12,6 km (`town_maquettes.json`), et Saint-Denis est à 9 km. Les lieux voisins tombent **dans** la
  maquette. La loi de collision (`min_scale`) réduit la plus petite, pas la plus grosse.
- Nature : design (ADR 0158, 0161).

**G2-7. Forêts de plaine en « poivre » et taches claires.**
Preuves : [`t_paris600_640.png`], [s1 `v03_nfr1000`], [s5 `v13_baltic400`].
- Ce qu'on voit : la plaine est criblée de points sombres réguliers. Les massifs de l'Île-de-France sont des taches vert-jaune
  **plus claires** que les champs, contraire à l'ADR 0710 (« une forêt se lit comme une masse sombre »). La Prusse, elle, est juste.
- Cause 1, les points : les arbres généralisés (ADR 0161) grossissent avec la distance (`campaign_forest_look.json`
  `far.grow_start_units 140 → 600`, `size_gain 0.7`). En plus, `generalised_far_density 0.35` garde un arbre isolé sur trois dans
  les haies et les bosquets : ce semis donne la trame de points.
- Cause 2, les taches claires : `forest_masses.floor 0.17` relève les masses sombres de la carte de couleur. La canopée y est plus
  claire que les cultures assombries par G1-2.
- Nature : réglage.

**G2-8. Routes en traits de rail, fleuves en fils.**
- Routes [s5 `v12_castile400`], [s6 `v14_calais250`], [s7] : traits blanc-crème rectilignes d'épaisseur écran constante
  (`line.wesl`, palier moyen 150-1200). Ils ressemblent à des voies ferrées. Une route de 1337 est un chemin de terre, souvent
  bordé d'arbres, invisible au-delà de quelques km, sauf les grandes voies.
- Fleuves [`t_paris600_640.png`], [s2] : `river_display.json` `major_min_px 4`. La Seine et la Loire, axes de mouvement
  (ADR 0343), se perdent sous d 200 et ne portent ni berge claire, ni ripisylve, ni plaine alluviale verte.
- Gironde [s4] : coin cyan triangulaire, artefact connu (« Gironde shape » dans la mémoire du lot Bevy).
- Nature : réglage et données.

### Moyen (G3)

**G3-1. Bascule 3D → parchemin sèche à 1 200 (fondu de 200).**
Preuve : [s1] : `v03_nfr1000` est en 3D, `v01_europe` est le parchemin.
- Ce qu'on voit : deux mondes sans transition visuelle. La 3D ne garde rien du parchemin (encre, papier), le parchemin rien de la
  3D (relief, couleur de sol).
- Cause : ADR 0124 (fondu croisé unique `ZoomTiers.strategic_weight`).
- Nature : design.

**G3-2. Parchemin en trame de « moisissure ».**
Preuve : [s1 `v01_europe`].
- Ce qu'on voit : les glyphes d'arbres à l'encre (`terrain.wesl::parchment_ground`) couvrent tout le continent d'un pointillé brun
  dense. Les frontières de royaumes sont lumineuses et multicolores (vert, jaune, rose) et se battent avec le lavis pastel.
- La belle vue parchemin des juges n'est vraie qu'avec moins de bruit.
- Nature : réglage.

**G3-3. Aucune nuit, aucune heure.**
- La campagne n'a qu'un « crépuscule doré pendant la résolution du tour » (`lighting.rs` `with_dusk`). Les six heures de la
  bataille (ADR 0755) n'existent pas en campagne.
- Le tour étant une saison, une nuit permanente n'a pas de sens. En revanche, une **aube de début de tour** et le crépuscule de fin
  de tour, mis en scène, donneraient le rythme.
- Nature : design.

**G3-4. Premier écran sous la pluie.**
- La partie s'ouvre sur la météo tirée par le cœur. À Paris, 370 gouttes actives et un brouillard de 31 à 149 u à d 25 (dump
  `v07_paris25.json`).
- Le premier contact visuel est le pire cas. Ce n'est pas un défaut de rendu, mais un défaut de mise en scène.
- Nature : design. Proposition : météo du premier tour dégagée sur la capitale du joueur, en règle de cœur, déterministe.

**G3-5. Brume lointaine trop claire.**
- La bande haute vaut 182-185 dans les Alpes et à Londres. Elle avale le dernier tiers de l'image en vue moyenne.
- Cause : `campaign_lighting.json` `aerial` (densité 0,85 en vue basse) plus `campaign_post.json` `aerial`.
- Nature : réglage, à reprendre avec le ciel (G1-1).

### Mineur (G4)

- **Liseré de terre crème sous les plaques de capitale** : `town_glow.json`, halo additif de 405 disques visibles. Joli de nuit,
  bruyant de jour.
- **Étendard rouge et blanc sur l'ost du joueur** [s2 `v06_paris60`] : à vérifier contre l'ADR 0155, « rouge réservé aux
  ennemis ». L'étendard peut être héraldique (oriflamme), mais il brouille la règle.
- **Le contenu « proche » est inaccessible** : paliers `valley` (8) et `site` (1,8) de `zoom_tiers.tres`, profil
  `close_camera.tres`, herbe sous 2,6, micro sous 0,02 d'empreinte. Tout cela est mort avec un plancher de 20. C'est du coût en
  code et en mémoire sans effet. À trancher : soit ouvrir le vrai gros plan, soit retirer.

---

## (c) Vision cible : bible de direction artistique de la carte de campagne

### Principe

**Une maquette de bois peinte posée sur une photo aérienne d'été, éclairée par un soleil de fin de matinée.**
- Le sol est vrai : couleurs et motifs d'une vue aérienne de campagne pré-industrielle.
- Les objets de jeu sont des maquettes grossies : villes, armées, châteaux.
- L'air est léger et la lumière chaude.
- Au dézoom, la peinture et l'encre du parchemin **montent** sur la même géographie au lieu de la remplacer.

Références de principe, à ne pas copier :
- les cartes de campagne Total War depuis *Three Kingdoms* et *Pharaoh* : sol peint par biome très lisible, horizon courbe et ciel
  en vue basse, frontières en bandes intérieures, plateau de jeu sous un ciel ;
- *Manor Lords* : parcellaire médiéval en lanières, haies, noyaux villageois, sentiers de terre ;
- *Old World* et *Humankind* : passage continu du terrain à la carte stylisée par le dézoom ;
- les vues aériennes réelles :
  - IGN « remonter le temps » 1950, avant le remembrement : openfield en lanières de Beauce et Picardie, bocage normand ;
  - Sentinel-2 en vraies couleurs, mai et septembre ;
  - photos de l'Alpe en juin : étages nets, neige blanche ;
- les cartes anciennes : carte de Cassini (forêts en masses vertes à lisière nette, villages en noyaux rouges, routes bordées
  d'arbres), portulans et Mappa mundi pour l'encre.

### Palette (albédo sRGB, printemps à midi, à poser dans `data/fx/palette.json`)

La règle qui manque aujourd'hui est la **hiérarchie de valeur**, du plus sombre au plus clair :
eau profonde < forêt < pré < labour < chaume et blé mûr < chemin et craie < villes et toits.

| Matière | Cible | Note |
|---|---|---|
| Forêt de feuillus (canopée de loin) | 0,16 0,24 0,11 | toujours plus sombre que tout champ ; résineux plus froids, 0,12 0,19 0,12 |
| Pré, prairie de fauche | 0,36 0,50 0,22 | herbe de référence de la bible |
| Blé d'hiver en herbe (printemps) | 0,42 0,55 0,24 | plus clair et plus jaune que le pré |
| Labour, jachère | 0,45 0,36 0,26 | terre brune ; craie de Champagne 0,62 0,58 0,50 |
| Chaume, blé mûr (été) | 0,70 0,60 0,36 | le clair de l'été |
| Vigne | 0,40 0,40 0,24 en rangs | lignes de 2 tons, teinte rouille en automne |
| Lande, garrigue, meseta | 0,55 0,47 0,30 | Castille ocre ; Écosse brun-violet 0,38 0,32 0,30 |
| Roche (calcaire / granite) | 0,62 0,60 0,56 / 0,45 0,44 0,43 | |
| Neige | **0,90 0,92 0,95** | au soleil ; ombre 0,62 0,68 0,80. La neige est le blanc du monde, la règle « jamais blanc pur » vaut pour le blanc 1,0 et pour la plaine |
| Mer, large | 0,06 0,16 0,22 | |
| Mer, plateau à moins de 50 m | 0,12 0,28 0,30 | |
| Mer, haut-fond et estran | 0,30 0,42 0,36 | sable et vase, sans cyan |
| Toits | tuile 0,55 0,30 0,20 ; ardoise 0,30 0,32 0,36 ; chaume 0,60 0,50 0,30 | les villages sont les seules taches chaudes du sol |

### Matières, par distance caméra (u = unité carte ≈ 719 m)

| Palier | Ce qui doit se lire | Ce qui le porte |
|---|---|---|
| d 20-60 (≈ 15-45 m/px) | quartiers de culture à stries (lanières suggérées), haies du bocage, noyau villageois (église, jardins), routes de terre bordées d'arbres, rivière avec berges et ripisylve, lisière forestière nette, figurines | parcellaire procédural + carte de couleur nette + maquettes Meshy |
| d 60-300 | soles de l'openfield (3 bandes de couleurs autour des villages), bocage en grain sombre, forêts en masses sombres, fleuves larges et brillants, villages en points chauds | parcellaire en patchwork, carte de couleur ; arbres isolés retirés |
| d 300-1 200 | grands pays : Beauce claire, Normandie verte et grenue, Landes sombres, Meseta ocre ; relief ; neige ; fleuves majeurs | carte de couleur rebâtie, relief, frontières en bande |
| > 1 200 | la même géographie qui devient carte : lavis de royaume, encre, papier | parchemin en surcouche de la 3D, fondu sur 900-1 600 |

### Lumière

- Soleil de **fin de matinée** : élévation 30-38°, 5 000-5 600 K, énergie au sol au moins **2 fois** celle du ciel.
- Ambiance de ciel moins bleue (0,75 0,80 0,90) et plus faible.
- Les ombres portées des maquettes sont la seule ombre dure ; le relief garde l'ombre cuite (ADR 0745).
- **Ciel** visible en vue basse : dégradé zénith → horizon de la bible, nuages de saison. La caméra descend à 18-22° au plus près
  pour montrer l'horizon (d < 60).
- **Brume** en une seule passe sur la profondeur, appliquée à la mer comme à la terre. Teinte = horizon du ciel. Jamais au-delà
  de 35 % du cadre en vue moyenne par temps clair.
- **Rythme du tour** : aube rose au début du tour (2-3 s, puis matin) et crépuscule doré à la résolution (existe déjà). Pas de nuit
  de jeu. La nuit sert aux événements (incendies, siège) à la demande.

### Saisons

Une saison **change des classes**, pas un filtre global.
- **Printemps** : blés vert tendre, prés vifs, vergers en fleurs (points blancs) près des villages, neige au-dessus de 1 800 m.
- **Été** : chaumes dorés, prés jaunis au sud, vignes vert sombre, neige au-dessus de 2 900 m.
- **Automne** : labours bruns, prés encore verts, forêts rousses et or, vigne rouille.
- **Hiver** : labours bruns, prés vert-olive pâle, forêts brun-violet, givre en plaques en plaine, neige franche en altitude et au
  nord ; drap blanc seulement par **météo** « neige ».

### Lisibilité stratégique (ordre de priorité visuelle)

1. Armées et villes : objets chauds, plaques.
2. Frontières : bande intérieure de 4-8 px aux couleurs de position ; aplat seulement en mode carte ou en parchemin.
3. Fleuves navigables et cols : lisibles et brillants.
4. Forêts et marais : couvert, embuscade, ADR 0045. Ce sont des masses sombres et vertes-brunes distinctes.
5. Routes : seules les grandes routes au-delà de d 150.
6. Le reste du sol est un fond calme.

---

## (d) Refontes proposées

Les coûts sont estimés en lots d'agent : S = demi-journée, M = 1-2 jours, L = 3-5 jours. Les fps sont visés en Ultra à 60
(16,7 ms), carte mesurée à environ 117 fps calme (juge technique iter-04). Tous les chiffres sont à confirmer au banc
`tools/bevy_bench`.

| # | Refonte | Effet visuel attendu | Coût | fps | Mémoire | Risques | ADR touchés |
|---|---|---|---|---|---|---|---|
| **R1** | **Ciel, lumière et air** : ciel procédural (Bevy `Atmosphere` à l'échelle carte, ou dôme dégradé + HDRI de bataille réutilisé), inclinaison basse au plus près (18-22°), soleil 30-38° et rapport soleil / ciel 2:1, brume unique en passe de profondeur appliquée à la mer, brume lointaine plafonnée, aube de début de tour | Fin du mur gris ; horizon, lumière chaude, mer et terre dans le même air ; +0,1 de valeur au sol | M | Atmosphère Bevy ≈ 0,3-0,6 ms ; dôme ≈ 0,05 ms ; passe de brume déjà payée | < 10 Mo (LUT) | Échelle de l'atmosphère avec 1 u = 719 m et relief ×4,3 ; nouveaux reflets de mer ; caméra basse = plus de relief et de tuiles à l'écran (charge CDLOD) | 0156, 0710, 0755 (amende), 0596, 0124 |
| **R2** | **Frontières et couleurs politiques en bande** : lavis de position retiré de la vue 3D sous d 900 (sauf modes carte), bande intérieure dégradée de 4-8 px à la couleur de position, hachures d'occupation gardées, aplat seulement en parchemin | Terrain des voisins lisible (Guyenne, Kent) ; lecture politique par les bords comme dans le genre | S | ≈ 0 (moins de remplissage) | 0 | Lisibilité du « à qui est cette province » sans aplat ; à compenser par la plaque et le survol | 0074, 0155, 0175 (amende) |
| **R3** | **Sol en photo aérienne médiévale** : poids de la carte de couleur au plus près 45 % → 10-15 % ; parcellaire procédural typé (openfield en soles et lanières autour des villages, bocage à haies, petites parcelles du Midi, terrasses, vignes sur adrets) à la place des facettes de Voronoi isotropes ; palette de cultures de la bible (c) ; noyau villageois (jardins, vergers) ; carte de couleur rebâtie avec la hiérarchie de valeur | Le sol cesse d'être du camouflage gris-bleu et devient un pays ; Beauce, Normandie, Castille distinctes | L | Coût cuit dans le cache de sol par nœud (ADR 0742) : ≈ +0,3 ms de cuisson au déplacement, ≈ 0 en régime | Carte de couleur rebâtie à la même taille (86 Mo BC1) ; paquet de parcelles inchangé | Nouveau look à valider par le joueur ; cohérence avec les règles d'agriculture (aucune : rendu seul) ; risque de damier si les bords sont trop durs (ADR 0787) | 0213, 0290, 0351, 0352, 0787 (remplacés en partie), 0142, 0358 |
| **R4** | **Montagne étagée et neige vraie** : étages par altitude réelle et exposition (fond de vallée vert, forêt de pente, alpage, éboulis, roche, neige blanche 0,9), glaciers lisibles, brume de vallée plafonnée par temps clair | Alpes, Pyrénées et Highlands crédibles et lisibles ; cols visibles | M | ≈ +0,1 ms (ALU, aucun échantillonneur : limite Metal de 16, `docs/wip/bevy-camp-samplers.md`) | 0 | Contredit « jamais blanc pur » (ADR 0710, 0750) : à amender pour la neige d'altitude seulement | 0710, 0750, 0353 |
| **R5** | **Eau : bathymétrie et rivières de carte** : bathymétrie réelle cuite (GEBCO, raster L8 ou R16 à 1/4) pour la couleur de la mer, liseré cyan retiré, estrans sable / vase ; fleuves navigables élargis (`major_min_px` 4 → 6-8 selon le débit), berges claires, ripisylve sombre peinte, plaine alluviale verte ; Gironde redessinée | Mer profonde et vivante, côtes naturelles, fleuves qui structurent la carte | M | ≈ 0 (un échantillonneur de plus dans `sea.wesl`, qui en a de la marge ; à vérifier) | +10-25 Mo | Source GEBCO (licence libre, à noter dans les crédits) ; génération outil `cent-ans geo` | 0163, 0750, 0215, 0496 |
| **R6** | **Villages et lieux génériques refaits** : 4 maquettes Meshy de la famille « ouest » (village, bourg, château, abbaye), toits de couleur régionale, noyau au sol, densité visible réduite au-delà de d 300 (village : 900 → 350), loi d'échelle des lieux voisins (la grosse maquette cède aussi) | Plus de « gravats » ; villages en points chauds lisibles ; Saint-Denis hors de Paris | M + 120 crédits Meshy | Moins d'instances en vue moyenne : gain probable | Textures 2048 BC1 ≈ 4 × 2,7 Mo | Cohérence avec les autres familles (TRELLIS) : écart de qualité visible en Italie et en Espagne | 0211, 0158, 0216, 0356 |
| **R7** | **Végétation de plaine** : arbres isolés retirés au-delà de d 150 (plus de « poivre »), masses forestières toujours plus sombres que les cultures (plancher relevé seulement pour la teinte, pas la valeur), haies du bocage en lignes sombres, saisons des arbres par classe | Forêts lisibles comme couvert stratégique ; plaine calme | S-M | Gain (moins d'imposteurs) | 0 | Perte de « vie » en vue moyenne, à compenser par le parcellaire de R3 | 0161, 0770, 0710 |
| **R8** | **Parchemin qui monte sur la 3D** : fondu étendu 900-1 600, la 3D garde relief et couleur de sol sous le lavis et l'encre, glyphes d'arbres éclaircis (densité ÷ 3), frontières de royaume en encre (une couleur par royaume, sans halo lumineux) | Un seul monde du zoom proche à l'Europe | M | Pendant le fondu, deux passes (≈ +1 ms sur 700 u de zoom) | 0 | ADR 0124 (« un seul fondu, pas de mélange ») à réviser | 0124, 0211 D4 |

**Saisons** (G2-4) : elles ne font pas une refonte à part. Elles sont réparties dans R3 (classes de culture par saison), R4 (neige)
et R7 (arbres). Le « drap blanc » d'hiver devient une météo (`plain_share` 0,93 → 0,25 hors météo neige).

---

## (e) Besoins en assets

### Meshy : 145 crédits, image → 3D meshy-6 texturé, ≈ 30 crédits l'unité

Les maquettes de lieux sont ce que le joueur voit le plus souvent, à toutes les distances de jeu. Les 8 villes emblématiques Meshy
prouvent l'écart de qualité avec TRELLIS. On remplace donc d'abord la famille **ouest** (France, Angleterre, Flandre), qui couvre le
théâtre principal de la guerre.

| Priorité | Modèle | Image source (Z-Image local, gratuit, deux vues : règle « objets orientés ») | Crédits |
|---|---|---|---|
| 1 | `village_west` : village nucléé du nord de la France, église romane à clocher, 12-15 maisons à colombage et toits de tuile et de chaume, enclos, mare, vergers | vue de trois quarts haute, fond neutre, éclairage diffus | 30 |
| 2 | `castle_west` : château à donjon carré, enceinte à tours rondes, basse-cour, sur motte légère | idem | 30 |
| 3 | `town_west` : bourg clos, halle, église gothique, faubourg | idem | 30 |
| 4 | `abbey_west` : abbaye cistercienne, église, cloître, granges, jardin clos, étang | idem | 30 |
| Reste | 25 crédits : réserve pour **une** reprise `meshy_retexture` (10) si un toit sort faux, ou un modèle `meshy-6-lite` (15) | | 25 |

À ne pas faire avec Meshy : arbres, rochers, sol, ciel. Les imposteurs existants suffisent.

Pas de soldat non plus : les figurines de campagne viennent de la bataille.

Prompts : respecter `docs/pipeline-assets-3d.md` § 2 (charte semi-réaliste, saturation moyenne ≤ 0,40). Les décrire **sans
négation**, car Z-Image ignore les négations.

### Z-Image : local, gratuit

1. **Textures de cultures vues du ciel, à stries**, pour le paquet `tx_campaign_parcels_pack.json` : blé vert, blé mûr, chaume,
   labour frais, jachère pâturée, pré de fauche, vigne en rangs, verger, lande à bruyère, meseta à chaume, rizière exclue.
   - Taille 1024², tuilable, vue verticale.
   - À utiliser surtout pour la **couleur moyenne et les stries** (R3), pas pour le grain.
2. **Planche de référence de palette** de la bible (c) : quatre vues aériennes peintes, une par saison, du même pays. Elle sert de
   cible pour les juges et pour la mesure HSV.
3. **Textures d'étages alpins**, vue verticale 1024² : alpage, éboulis calcaire, éboulis granitique, névé.
4. **Glyphes d'encre d'arbre et de montagne** pour le parchemin (R8), en remplacement du pointillé : 8 variantes, fond transparent
   après détourage.
5. **Images sources des 4 maquettes Meshy** ci-dessus : 2 vues chacune, best-of-4.

Pour le ciel : pas de génération. On réutilise les HDRI des batailles (ADR 0017), qu'on décline en dégradé de données, ou
l'atmosphère procédurale de Bevy.

---

## (f) Ordre recommandé

1. **R2 Frontières en bande** (S) : gain de lisibilité immédiat, risque nul. Les captures de tout le reste deviennent jugeables.
2. **R1 Ciel, lumière, air** (M) : change toutes les images ; prérequis pour juger la palette du sol.
3. **R4 Montagne et neige** (M) : réglages et ALU, effet spectaculaire.
4. **R3 Sol médiéval** (L) : le gros morceau, à faire sur la lumière de R1, en deux lots (carte de couleur rebâtie, puis
   parcellaire typé).
5. **R7 Végétation de plaine** (S-M) : avec R3, pour que le parcellaire respire.
6. **R5 Eau** (M).
7. **R6 Lieux génériques** (M + Meshy) : images Z-Image d'abord, choix du joueur, puis Meshy.
8. **R8 Parchemin qui monte** (M) : en dernier, quand la 3D vaut d'être gardée sous le parchemin.

Après chaque refonte : `tools/compare_views.sh` sur les vues de contrôle, mesure HSV (`out/audit/visuel/stats.py` à promouvoir
dans `tools/refshots/`), juge DA, banc fps.

Proposition d'ajout permanent à `tools/refshots/views.txt` : `camp_guyenne 0300` (-0.57 44.84 300), `camp_alps 0120`
(7.0 45.85 120), `camp_baltic 0400` (18.6 54.4 400), `camp_castile 0400` (-3.7 42.3 400), `camp_calais 0250`
(1.85 50.95 250). Toutes en `CENT_ANS_WEATHER=clear`, plus une vue d'hiver et une de brouillard.

---

## (g) Specs de lots pour les trois refontes prioritaires

Règles communes à tous les lots :
- worktree dédié, squelette commité d'abord, note `docs/wip/<lot>.md` ;
- commandes lourdes par `tools/heavy.sh` ;
- client lancé seulement par `client/run_bg.sh` avec `CENT_ANS_MUTE=1` et `--offscreen` ;
- avant commit : `cargo fmt`, `clippy -D warnings`, `cargo test --features dev`, `shader_check` vert ;
- smoke à l'intégration ;
- aucune valeur codée en dur : tout réglage nouveau va dans les données, avec son schéma.

Les agents d'implémentation ne prennent pas de capture. Une session ou un agent dédié fait la vérification visuelle (5 captures
au plus).

### Lot R2 : FRONTIÈRES-BANDE (S, agent Sonnet possible)

**But** : en vue 3D, la position diplomatique se lit par une bande intérieure dégradée, pas par un aplat.

**Fichiers** :
- `data/map/stance_fill.json` et son schéma (`data/schemas/`) :
  - nouveau bloc `band` : `width_px` [4, 8] (vue proche / moyenne), `alpha` par position, `falloff` ;
  - `zoom.flat_from_distance` (défaut 900) : l'aplat ne revient qu'à partir de là, et pleinement en parchemin.
- `client/src/campaign/provinces/shaders/fill.wesl` : bande intérieure. Le lavis est dosé par la distance au bord de la province,
  estimée par **4 lectures voisines** à 2 rayons (8 taps) du raster des provinces déjà lié ; aucun échantillonneur ajouté.
  - Variante si le coût dépasse 0,2 ms : distance au bord cuite hors ligne (outil `cent-ans geo`, raster R8 au quart).
- `client/src/campaign/provinces/overlay.rs` : passage des nouveaux paramètres ; le mode `political` garde l'aplat seulement si
  `strategic > 0` ou si `distance > flat_from_distance`.
- Lentilles (religion, loyauté, ravitaillement…) : **inchangées**. Elles sont choisies par le joueur.
- ADR : amendement de 0175 (nouvel ADR « position diplomatique en bande intérieure en vue 3D »).

**Tests** :
- test unitaire `stance_fill_band_defaults_parse` ;
- test pur de la fonction d'alpha par distance (0 au cœur de la province sous d 900, valeur du `band.alpha` au bord) ;
- `shader_check`.

**Critères d'acceptation** :
1. Vue `camp_guyenne 0300` claire : la saturation et la valeur moyennes de la Guyenne (zone 30 % au centre de la province)
   restent à ± 15 % de celles de la Saintonge voisine (mesure `stats.py` sur un recadrage).
2. Bande rouge visible au bord, d'au moins 4 px.
3. En parchemin (d 2 600), l'image reste inchangée (comparaison avec `v01_europe`).
4. Temps GPU : ≤ +0,2 ms (banc pan d 300).

**Vues de contrôle** : `camp_guyenne 0300`, `camp_calais 0250`, `camp_2600`.

### Lot R1 : CIEL-LUMIÈRE (M, agent Opus ou cent-ans-dev)

**But** : un ciel et un horizon en vue basse, une lumière chaude de fin de matinée, une seule brume pour la terre et la mer.

**Fichiers et données** :
1. **Ciel**.
   - D'abord, prototype de l'`Atmosphere` de Bevy 0.20 sur la `CampaignCamera` : réglage d'échelle scène → mètres (vérifier le
     nom exact du champ dans 0.20). Mesurer coût et rendu. Un horizon trop bas ou trop haut à cause du relief ×4,3 fait échouer
     ce critère.
   - Repli : dôme dégradé simple, avec zénith, horizon et brume de `palette.json` `sky`, nuages de saison, et l'HDRI de bataille
     en option.
   - Code : `client/src/campaign/camera/mod.rs` (fin de `clear_color` gris) et nouveau `client/src/campaign/terrain/sky.rs` ou
     `weather/sky.rs` (existe : cirrus, à étendre).
   - Données : `data/fx/campaign_sky.json` avec schéma.
2. **Caméra** : `camera/rig.rs`, `pitch_near_deg` 30 → donnée (`data/ui/camera_feel.json`), cible 20° à d 22 et 28° à d 150. La
   pente reste linéaire jusqu'à 70° à 2 600.
   - Vérifier que l'horizon entre dans le cadre à d ≤ 60.
   - Vérifier le streaming CDLOD : `lod.rs` `MAX_PAGES` et les trous.
3. **Lumière**.
   - `data/fx/atmosphere.json` `campaign.seasons.*` : `sun_elevation` 32-36, `sun_energy` ≈ 2,4, `sun_color` vers 5 200 K.
   - `data/fx/campaign_lighting.json` : `ambient.color` [0,75 0,80 0,90], `energy` ≈ 0,40, `elevation_bounds_deg` [26, 40].
   - Garder `with_dusk` de fin de tour. Ajouter une `dawn` de début de tour (2,5 s, données).
4. **Brume unique**.
   - `campaign/terrain/post.wesl` porte la brume météo et la brume de vallée sur la profondeur, donc la mer comprise.
   - Retirer ou neutraliser le voile de brouillard peint dans `terrain.wesl` (`weather_ground` brume) quand la passe post est
     active.
   - `sea.wesl` lit la même teinte d'air.
   - Plafond de brume lointaine par temps clair : `campaign_post.json` `aerial.clear_max`.
5. **Palette** : `data/fx/palette.json` reçoit les cibles de campagne de la bible (c), avec un test de tenue.
6. **ADR** : nouvel ADR « ciel et lumière de la carte » (amende 0156, 0710, 0755).

**Tests** :
- `palette::campaign_and_battle_share_the_common_grade` toujours vert ;
- nouveau test « l'éclairement soleil au sol est au moins 1,8 fois celui du ciel pour chaque saison » (données) ;
- test de rig « à d 22, le rayon du haut de l'écran passe au-dessus de l'horizon » ;
- dump `campaign_weather.atmosphere` complété par `sky.kind` et `sky.visible`.

**Critères d'acceptation**, mesurés sur captures claires (`CENT_ANS_WEATHER=clear`) :
1. `camp_0060` (Paris d 60) : sol de teinte 82-106°, saturation ≥ 0,22, valeur 0,38-0,55.
2. Bande haute de l'écran : teinte de ciel (B > R + 0,08) à d 25 et 60.
3. `v17` brouillard : la luminance de la mer et celle de la terre sous brume restent à ± 10 % l'une de l'autre, à distance égale.
4. Alpes d 400 : moins de 35 % du cadre plus clair que 0,75 (pas de voile).
5. Banc pan d 30 Ultra : p95 ≤ 16,7 ms ; écart ≤ +0,8 ms.

**Vues de contrôle** : `camp_0025`, `camp_0060`, `camp_0600`, `camp_alps 0120`, `v17` (brouillard), `v15` (hiver).

### Lot R3 : SOL-MÉDIÉVAL (L, deux sous-lots, Opus pour la conception, Sonnet pour l'outil)

**But** : le sol se lit comme une vue aérienne de campagne de 1337, avec un parcellaire typé par pays et une palette de cultures
vraie.

**R3a : carte de couleur rebâtie (outil Python, Sonnet)**
- `data/map/colormap_style.yaml` : palette de la bible (c), avec hiérarchie de valeur forêt < pré < labour < chaume.
  - `mosaic.crop_contrast` relevé.
  - Classes Meseta, Landes, Beauce et bocage distinctes, à partir de `agri_landscapes.json` et `agri_regions.png`.
- `tools/cent_ans_tools/` commande `geo colormap` : même taille (scale 2) ; noyaux villageois (jardins, vergers) autour des
  3 900 lieux indexés ; routes moins claires (`road` vers 0,55 0,48 0,36).
- Tests : `uv run --project tools pytest` (tests existants de la carte de couleur) et nouveau test de palette (la luminance
  moyenne des classes est strictement ordonnée).
- La carte de couleur est une donnée liée hors dépôt (`docs/editeur-carte.md`). Il faut régénérer, puis livrer par le paquet de
  relief (ADR 0149 bis).

**R3b : parcellaire typé dans le shader (Opus)**
- `client/src/campaign/terrain/terrain.wesl` (`regional_ground`, `fields_apply`) :
  - poids de la carte de couleur au plus près 0,45 → donnée `ground_biome_mix.json` `view.colormap_near_share` (cible 0,12) ;
  - les facettes de Voronoi isotropes deviennent des **cellules étirées et orientées** : grille déformée, rapport 1:4 à 1:10, cap
    par sole.
  - Par type de paysage (`agri_landscapes.json`), le parcellaire prend l'une de ces formes :
    - `openfield` : 3 soles en couronnes autour du noyau villageois le plus proche (index des villages déjà chargé par
      `countryside/terrain_data.rs`, à passer en texture de distance cuite si besoin), stries de lanières sous-pixel en anisotropie
      à d ≤ 40 ;
    - `bocage` : cellules irrégulières avec haie sombre de 1-2 px à d ≤ 80 ;
    - `midi` : petites parcelles, vigne et olivier sur adrets (règle ADR 0352 gardée) ;
    - `montagne` : prés de fauche et alpage.
- Classes de culture par saison, et non par filtre : `season_field` par classe, teintes de la bible (c).
- `data/art/ground_biome_mix.json` et `data/fx/ground_detail.json` avec leurs schémas : nouveaux blocs `landscapes.<type>`
  (rapport, taille de cellule en m, largeur de haie, part de chaque culture par saison).
- Coût : tout passe par le cache de sol par nœud (ADR 0742). Aucun échantillonneur ajouté, limite Metal de 16.
- ADR : nouvel ADR « parcellaire typé » (remplace 0213 et 0787 pour la forme, amende 0290, 0351, 0352).

**Tests** :
- `shader_check` ;
- tests purs Rust des fonctions de parcellaire portées en CPU (`cpu_sampler.rs` sert déjà de miroir) : orientation stable, part
  de chaque culture à ± 5 % de la donnée sur 10 000 tirages ;
- `ground_pack_manifests_have_no_missing_or_pink_layer` toujours vert ;
- dump `campaign_terrain.material.fields` complété par le type de paysage au foyer.

**Critères d'acceptation**, en temps clair, sur des captures où le sol est bien lié (`regional_ground.bound = true`) :
1. **Contraste entre cultures** : une k-moyennes à 4 classes sur la bande sol de `camp_0060`, mesurée sur l'image en sortie de
   rendu, donne au moins 3 classes séparées de ΔE ≥ 8.
2. **Pas d'arêtes polygonales** : un juge DA ne cite plus « facettes », « camouflage » ni « damier ».
3. **Régions distinctes** : Castille d 400, teinte moyenne 45-70° ; Normandie d 400, 85-105°.
4. **Saisons par classe** : en automne, la Loire d 80 a une valeur ≥ 0,38 et au moins 2 classes chromatiques (vert de pré et brun
   de labour).
5. **Performance** : cuisson par nœud ≤ +0,3 ms en moyenne au pan ; p95 Ultra ≤ 16,7 ms.

**Vues de contrôle** : `camp_0025`, `camp_0060`, `camp_0200`, Loire d 80 (printemps et automne), `camp_castile 0400`,
`v15` hiver.

---

## Annexe : vues prises

| Vue | Caméra (lon, lat, d) | Réglage | Planche |
|---|---|---|---|
| `t_paris600` | 2,35 48,85 600 | — | lue seule |
| `v01_europe` | 8 50 2600 | — | s1 |
| `v03_nfr1000` | 1,5 49,3 1000 | — | s1 |
| `v05_paris200`, `v06_paris60` | Paris 200, 60 | pluie du tour 1 | s2 |
| `v07_paris25` | Paris 25 | pluie | lue seule à 640 px |
| `v08_guyenne300` | -0,57 44,84 300 | — | s4 |
| `v09_alps400`, `v10_alps120` | 6,86 45,9 / 7,0 45,85 | — | s3 |
| `v11_scot400` | -3,2 56 400 | — | s4 |
| `v12_castile400` | -3,7 42,3 400 | — | s5 |
| `v13_baltic400` | 18,6 54,4 400 | — | s5 |
| `v14_calais250` | 1,85 50,95 250 | — | s6 |
| `v15_paris200_winter` | Paris 200 | `CENT_ANS_SEASON=winter` | s6 |
| `v16_loire80_autumn` | 0,69 47,39 80 | `CENT_ANS_SEASON=autumn` | s7 |
| `v17_normandy200_fog` | 0,1 49,2 200 | `CENT_ANS_WEATHER=fog` | s7 |
| `v18_normandy200_storm`, `v19_london120_eng`, `v20_loire80` | — | orage ; Angleterre ; printemps | mesures seulement |
| `c1_paris60_clear`, `c4_montstmichel40_clear` | — | `CENT_ANS_WEATHER=clear` | s8 |
