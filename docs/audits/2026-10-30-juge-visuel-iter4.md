# Juge visuel frais, boucle juge 2, itération 4 (30/10/2026)

État jugé : `main` 462660a15 (FUSION-B : VIS-R4 montagne étagée et neige, VIS-L1 sol photo, VIS-L2 habitat à l'échelle, VIS-L3
brume unique et nuages, VIS-L6 sol de bataille et chevaliers, compteurs PERF). Client Bevy `--features dev`, graine 1337, France,
printemps 1337, temps clair. Worktree propre au juge, hors écran et muet (`tools/compare_views.sh`, `client/run_bg.sh`).

Captures : les 19 vues de `tools/refshots/views.txt`, plus 3 vues ajoutées localement (`camp_0900` Beauce d 900, `camp_1400`
Touraine d 1400, `camp_0030_loire` d 30), plus 6 captures de mêlée (`CENT_ANS_UNITS_SPEED=16,1`, `CENT_ANS_BATTLE_MELEE=28|40`,
Crécy par défaut) avec dump `battle_units`. Toutes les images sont mesurées par script (moyenne sRGB puis HSV par boîte, médianes
par pixel, part de pixels par classe) ; 10 images lues à 640 px (dont 2 planches 2 × 2). Rien n'est commité hors ce fichier.

## Note

| Domaine | Iter 3 | Iter 4 | En une phrase |
|---|---|---|---|
| Carte de campagne 3D (d 25-900) | 4/10 | **5/10** | Le gris est vaincu et les pions géants ont disparu, mais on a troqué la brume grise contre une pelouse uniforme où les forêts sont plus claires que les prés. |
| Parchemin (d 1400-2600) | 5/10 | **3,5/10** | Régression : lavis orange vif, frontières jaune et vert fluo ; le gain de saturation ×2,8 de VIS-L1 s'applique aussi au parchemin. |
| Montagne (VIS-R4, première évaluation) | — | 5,5/10 | Les étages et les crêtes ont enfin une forme ; la neige est jaune beurre (s 0,29-0,37). |
| Bataille | 6/10 | **6/10** | Les caparaçons fleurdelisés et les morts au sol donnent enfin une mêlée crédible ; mais l'image se postérise au bout de quelques minutes, et le ciel est surtraité. |
| **Global** | **4,5/10** | **5/10** | Un vrai pas (couleur, échelle, relief) ; mais un réglage global de saturation crée trois défauts neufs. Le sol n'a toujours ni dessin de photo aérienne ni identité régionale. |

## Mesures contre la bible de palette

Méthode : boîtes nommées sur la capture 1280 × 800. « s50 / v50 » = médiane par pixel ; « moyenne » = HSV de la couleur moyenne
(méthode de l'iter 3). « vert sombre / clair » : parmi les pixels verts (h 50-150°, s > 0,08) du monde hors interface, moyenne de v du
quintile le plus sombre contre celle des 30 % les plus clairs. C'est une approximation du rapport forêt / pré, à défaut de masque.

| Zone (vue) | Iter 3 h / s / v | Iter 4 h / s / v | Cible (bible ou critère du lot) | Verdict |
|---|---|---|---|---|
| Plaine, bande milieu (camp_0600) | 67° / 0,20 / 0,50 | 73° / 0,50 / 0,60 (s50 0,47, v50 0,64) | s ≥ 0,45 ; pré v 0,50 | s tenue ; **trop clair** (v 0,64) |
| Plaine (camp_0200) | 90° / 0,20 / 0,41 | 78° / 0,42 / 0,56 (s50 0,45) | s ≥ 0,45 | limite |
| Plaine (camp_0300 / calais / guyenne) | — | s50 0,42 / 0,44 / 0,46 ; v50 0,54-0,60 | s ≥ 0,45 | limite, 2 vues sur 3 juste en dessous |
| Bocage normand (vis_r3b_normandie) | 123° / 0,21 / 0,48 | 80-83° / 0,46 / 0,56 (s50 0,41) | 89° / 0,56 / 0,50 | menthe corrigée ; s sous la cible |
| Beauce d 900 (camp_0900) | — | 72° / 0,50 / 0,61 (s50 0,49, v50 0,65) | pré v 0,50 | trop clair, mouchetis brun |
| **Forêt contre pré (camp_0200, boîtes)** | v 0,39-0,40 contre 0,41-0,50 | **forêt v 0,53-0,62 contre pré 0,44-0,47** | v forêt / v pré ≤ 0,6 (bible 0,48) | **inversé : rapport 1,2-1,4** |
| Vert sombre / clair (toutes vues 3D) | — | 0,51-0,72 (0200 : 0,56 ; loire : 0,72 ; calais : 0,67) | ≤ 0,6 | non tenu dans 6 vues sur 11 |
| Parcelles claires (labour, chaume) d 25-60 | 0 % au-dessus de v 0,45 | 13 % (camp_0060), 17 % (d 30 Loire), 27 % (camp_0025) | ≥ 15 % à d 60 | presque tenu |
| Castille d 400 (vis_r3b_castile) | 96° / 0,13 / 0,46 | meseta 88° / 0,38 / 0,53 | ocre de meseta h 35-55° | **vert de prairie normande** |
| Highlands d 400 | — | lande 81° / 0,45 / 0,56 | bruyère h 20-45°, s 0,25-0,4 | **pelouse verte** |
| Ciel, bande haute (camp_0025 / 0060 / loire / wissant / alpes120) | 211-214° / 0,13-0,18 | 209-217° / 0,18-0,25 / 0,82-0,91 | s ≥ 0,25 | **non tenu** (1 vue sur 5) |
| Bande de ciel et brume (d 25-60) | 20-25 % du cadre | 11-14 % du cadre | ≤ 12 % | presque tenu |
| Neige (vis_r4_alpes400 / alpes120 / pyrenees300, pixels v > 0,8) | — | **h 47-50° / s 0,29-0,37 / v 0,92-0,95** | neige s ≤ 0,10 | **jaune beurre** |
| Sol de bataille près (battle_low) | 80° / 0,24 / 0,53 | 86° / 0,60 / 0,54 | s ≥ 0,38, v ≤ 0,50 | s tenue, v un peu haute |
| Sol de bataille loin (battle_low) | 59° / 0,18 / 0,58 | 52° / 0,20 / 0,65 | — | toujours délavé |
| Ciel de bataille (battle_low) | — | 231° / 0,37 / 0,65 (s50 0,46) | ciel de jour v ≥ 0,7 | **sombre et surtraité** (effet « HDR ») |
| Chevaliers en mêlée, hors herbe (mb_a, 28 m) | 250° / 0,04 / 0,47 | moyenne 337° / 0,12 / 0,45 ; s50 0,22 ; 45 % des pixels ≥ 0,25 | s moyen ≥ 0,25 | **non tenu** à la mesure ; caparaçons bleus fleurdelisés visibles |
| Lavis du parchemin (camp_1400, coin orange) | — | 31° / 0,35 / 0,87 | ≤ 0,6 sur les frontières (L5) | fonds orange et jaune fluo |

Constat transversal. VIS-L1 a relevé `legacy_adjust.saturation` de 1,16 à **2,8** (`data/fx/campaign_post.json`). Dans
`client/src/campaign/terrain/post.wesl` (fragment, ligne ≈ 224), ce gain s'applique **à tout l'écran**, sans le `master` qui éteint
l'atmosphère en vue parchemin. Il touche donc le sol, mais aussi le parchemin, la neige, la mer, les bannières et les maquettes. Un
blanc chaud (neige `[0.70, 0.68, 0.64]`, s 0,09) sort à s 0,3 : c'est la neige beurre. Les lavis politiques deviennent fluo. Le
gain extrapole par rapport à la moyenne des trois canaux, sans tenir compte du matériau. C'est la cause commune de trois défauts G1
de cette itération. Il faut le remplacer par une chroma juste à la source, pas le régler.

## Contrôle des lots de l'iter 3 tels que fusionnés

| Lot | Critère | Résultat |
|---|---|---|
| L1 sol | plaine s ≥ 0,45 | limite : s50 0,41-0,49 selon la vue |
| L1 sol | v forêt / v pré ≤ 0,6 | **non tenu, inversé** à d 200 (forêts plus claires que les prés) |
| L1 sol | plus de peigné à d 150-400 | tenu : le peigné a disparu ; mais le préfiltre a tout lissé en pelouse uniforme, sans parcellaire lisible à d 200 |
| L1 sol | lanières texturées à d 25 | non fait (Z-Image reporté, ADR 0826) : aplats vectoriels à bords nets en éventail (crop de camp_0060) |
| L2 habitat | plus de pions géants | **tenu** : aucune maison isolée géante à d 150-300 |
| L2 habitat | décalque `hamlet_core` | **non vu à l'écran** dans camp_0025, 0060, 0200 et calais (rayon 220 m, retiré au-delà d'une distance de rig de 260). Dans le code (`countryside/hamlet_core.rs`, `core_pixels`), c'est une **texture de 128 px générée au chargement** : bruit de valeur et points de toit. C'est un visuel fait par le code, contraire à la règle et à la spec, qui demandait des décalques Z-Image. À remplacer. |
| L2 habitat | plus de géométrie bâtie par le code | tenu pour `towns/shapes.rs`. Mais les **prismes verts extrudés** à côté des fermes restent à d 25-60. Ce sont les modèles de champ générés (`dn_fields.json`, ADR 0222), qui se lisent en low poly. |
| L2 habitat | échelle des figures | partiel : le chariot fait la taille de la ferme ; le cavalier d'armée fait un tiers de Notre-Dame (camp_0025) |
| L3 air | brume ≤ 12 % | presque tenu (11-14 %) |
| L3 air | ciel s ≥ 0,25 | **non tenu** (0,18-0,25) ; nuages de campagne à peine perceptibles à d 25-120 |
| L3 air | aucune étiquette au-dessus de l'horizon | **non tenu** : « Hainaut, Brabant, Lorraine, Palatinat du Rhin, Souabe » flottent dans le ciel (alpes120) ; « Escaut, Meuse, Rhin » au-dessus de Paris (camp_0025, camp_0060). L'ADR 0833 renvoie ce point à `markers/` (non fait). |
| L6 bataille | sol près s ≥ 0,38 | tenu (0,60) |
| L6 bataille | chevaliers s ≥ 0,25 | **non tenu** à la mesure (s50 0,22) ; à l'œil, les surcots et les caparaçons aux armes de France sont là |
| L6 bataille | silhouettes variées | tenu à l'œil (taille, robes, poses) |

## Triangles de la mêlée : 6,34 M → 3,17 M

Conclusion : **artefact du banc, pas une perte de soldats.** Preuves :
1. Les rapports du lot PERF avant fusion (`agent-ab8b1b69…/out/bench/battle_b1..b3, guard`) contiennent déjà les deux régimes.
   Avec `contact: true` (contact à 550 s de simulation), on a 6,33-6,43 M triangles, 441-444 appels de dessin et 35 300 instances.
   Avec `contact: false` (b1, 474 s), on a **3,47 M, 401 appels de dessin et 27 900 instances**. Les valeurs recalibrées après
   FUSION-B (3,17 M, 383 appels de dessin) sont celles du régime sans contact.
2. Cause : `bench_battle.rs` arrête l'avance rapide après `CONTACT_TIMEOUT_FRAMES = 3 000` **images**, pas après un temps de
   simulation. À x16, une machine moins chargée (images plus courtes) atteint le délai avant le premier contact (≈ 550 s de
   simulation) ; la caméra vise alors le barycentre de toutes les unités, loin de la mêlée.
3. Rien n'a changé dans le LOD : `LOD0/LOD1/IMPOSTOR_DISTANCE` (16/75/300 m) n'ont pas bougé depuis le portage ; VIS-L6 ne touche
   qu'au teint, à la taille par instance et au caparaçon. En mêlée réelle à 28 m (`mb.json`), 2 262 soldats sont présents, avec
   28 en LOD complet, 446 moyens et 1 788 lointains ; on voit chevaux, cavaliers, lances, morts et blessés au sol. Il ne manque
   aucune pièce d'armure à l'œil.
4. À corriger dans le banc (lot technique) : délai en secondes de simulation (≥ 900 s), et garde en échec si `contact` est faux.
   Sinon la référence de 3,17 M masque toute régression réelle de la mêlée.

## Défauts

### G1 (flagrant)

**G1-1. Saturation globale ×2,8 : parchemin fluo, neige beurre.** (`camp_1400`, `camp_2600`, `vis_r4_alpes400/120`,
`vis_r4_pyrenees300`)
- En vue parchemin, les lavis politiques sont en orange vif, jaune citron et gris-mauve, avec des frontières en double trait jaune et
  vert fluo. C'est plus criard qu'à l'iter 3.
- La neige des Alpes et des Pyrénées est crème-jaune (h 48°, s 0,3), comme du beurre ou du vieux papier ; la ligne de neige se lit
  comme une zone de calcaire jauni.
- Cause : `legacy_adjust.saturation 2.8` appliqué à tout l'écran par `post.wesl`, hors du `master` du parchemin.

**G1-2. Forêts plus claires que les prés.** (`camp_0200`, `camp_0300`, `vis_r3b_loire`)
- Les massifs forestiers sont des mousses vert-jaune duveteuses, avec v 0,53-0,62. La prairie autour est à v 0,44-0,47. Toute la
  hiérarchie de valeurs d'une photo aérienne (forêt = masse sombre) est inversée. C'était le défaut G2-1 de l'iter 3 ; le gain
  de chroma l'a aggravé.
- Le critère de VIS-L1 (≤ 0,6) n'est pas vérifiable par le lot : il n'a pas de masque de forêt dans ses mesures.

**G1-3. Sol en aplats vectoriels et prismes verts à d 25-60.** (`camp_0025`, `camp_0060`, `camp_0030_loire`)
- Les lanières sont des polygones plats, bruns et verts, à bords francs. Elles rayonnent en éventail avec des traînées étirées et
  n'ont ni sillon, ni grain, ni haie. Au pied d'une cathédrale Meshy très détaillée, le contraste de style est violent.
- À côté de chaque ferme, des prismes vert vif extrudés (modèles de champ générés) se lisent en low poly, ce que le joueur a
  exclu.

**G1-4. Étiquettes et figures au-dessus de l'horizon.** (`vis_r4_alpes120`, `camp_0025`, `camp_0060`)
- Des noms de régions à 300-600 km flottent dans le ciel bleu, au-dessus des crêtes. C'est le défaut G1-3 de l'iter 3, non traité
  (renvoyé par l'ADR 0833).
- Le cavalier d'armée et sa bannière font un tiers de la hauteur de Notre-Dame.

**G1-5. Bataille : l'image se postérise au bout de quelques minutes.** (`melee_600`, `melee_2400`, `mb_b`, `mb_c`)
- Dans une même exécution, la capture à 2 300 images est saine (`mb_a`). À 2 360 et 2 420 images (`mb_b`, `mb_c`), l'image devient
  une affiche en aplats : 39-43 % de pixels noirs, 43-50 % de pixels blancs, 18-20 % de pixels à s > 0,9.
- Sur 6 captures en marche, 4 sont touchées. Les 2 captures de `compare_views` (bataille en pause) sont saines. La note VIS-L6
  avait mis ce symptôme sur le compte d'une recompilation ; ici, rien n'a été recompilé (build à 0,23 s).
- Le dump `postfx` exclut l'exposition auto et l'accentuation (`hdr: false`). Piste à suivre : un NaN ou un infini qui se
  propage dans un matériau (sang, fumée, poussière de mêlée, SSAO) une fois la simulation avancée.
- Si le joueur le voit, c'est le pire défaut du jeu. Priorité 1 d'investigation (lot M5).

### G2 (majeur)

- **G2-1. Le sol n'a pas d'identité régionale.** La Castille au printemps (h 88°) a la même pelouse que la Normandie et les
  Highlands (h 81-83°). La Beauce d 900 est un tapis piqué de points bruns, comme un tapis brossé. Aucune région ne se reconnaît sans
  ses plaques.
- **G2-2. À d 200-300, pelouse lisse.** Le préfiltre de VIS-L1 a tué le peigné en fondant tout dans la carte de couleur : il n'y a
  plus de parcellaire, de haies ni de noyaux villageois lisibles. On passe du hachis au gazon de golf.
- **G2-3. Ciel de bataille surtraité.** Bleu marine dense (v 0,65, s50 0,46), cumulus blancs à fort contraste : cela se lit comme
  une photo HDR de téléphone, pas comme un ciel peint. Le sol lointain reste délavé (s 0,20, v 0,65).
- **G2-4. Lignes droites qui traversent la mêlée** (`mb_a`) : de longs traits rouges et blancs, rectilignes, sur 100 m et plus,
  convergent vers la droite à travers les cavaliers ; une ligne blanche part du bas vers l'horizon gauche. Il peut s'agir de
  trajectoires de flèches, d'ordres ou de lances étirées, à identifier. On ne les voit pas en vue calme.
- **G2-5. Chevaliers encore ternes à la mesure** (s50 0,22). La couleur héraldique est noyée par l'acier et la robe des chevaux. Le
  critère « s moyen de la couleur moyenne » est trop dur ; garder la part de pixels « étoffe » à s ≥ 0,35 (cf. M5).
- **G2-6. Maquettes de ville absentes à d 300-400** (Bordeaux, Caen, Nantes, Rouen) : toujours de simples plaques (G2-8 de
  l'iter 3, non traité).

### G3 (moyen)

- La colonne blanche verticale de fumée en trait reste à gauche du moulin (battle_low, mb_a).
- Les ombres des arbres de campagne à d 25-60 sont bleu-violet saturé, conséquence du gain ×2,8 sur les ombres froides.
- La Manche est désormais bleue et vive (VIS-L4 en cours) : c'est bien, mais la côte normande bascule en cyan au contact.
- Le contour rouge vif de l'enclave anglaise (Kent et Calais) reste plus voyant que les vraies frontières.

## Lots de refonte proposés

VIS-L4 (eau) et VIS-L5 (plaques et parchemin) sont en cours. Critères ajoutés pour eux à la fin. Ordre conseillé : **M5 (étape a,
diagnostic) tout de suite**, puis M1 avant tout le reste (il change la couleur de tout), puis M2 et M3 en parallèle (ils touchent des
modules différents du shader de sol depuis le découpage de l'ADR 0826), puis M4 et M6.

### M1 CHROMA-À-LA-SOURCE : supprimer le gain global, étalonnage par matériau (M)
- Spec. (a) `legacy_adjust.saturation` revient à ≤ 1,15. Le bloc de saturation de `post.wesl` passe sous le `master` (zéro sur le
  parchemin) et protège les blancs et les neutres : pas de gain sous une chroma de 0,06, et les blancs chauds restent blancs.
  (b) La chroma perdue est rendue dans l'albédo, **par classe de matériau**, par une table de cibles sortie de la bible de palette
  (`data/fx/palette.json` : pré, forêt, labour, chaume, lande, roche, neige, eau). Ce n'est plus un gain d'écran. La conversion
  AgX est compensée une fois par une LUT 3D cuite hors ligne (`tools/`, 32³), mesurée sur une planche de nuanciers rendue par le
  vrai pipeline. (c) Ajouter un **test de nuancier** : une scène de test, une capture et une mesure automatique des HSV des
  nuanciers contre la bible (± 8° de teinte, ± 0,06 de s et de v), lancé par `compare_views` (nouvelle vue `swatches`).
- Critères. Neige v > 0,8 : s ≤ 0,10. Parchemin : aucun lavis à s > 0,5, frontières à s ≤ 0,6. Plaine s ≥ 0,45 sur
  camp_0200/0600 sans saturation d'écran > 1,15. Ombres d'arbres à s ≤ 0,3. Test de nuancier vert.
- Fichiers : `client/src/campaign/terrain/{post.wesl, post.rs, ground_look.wesl}`, `data/fx/{campaign_post.json, palette.json}`,
  `tools/` (LUT et mesure), `tools/refshots/views.txt`, ADR.
- GPU : neutre (une LUT 3D en plus, un gain en moins). Assets : aucun.

### M2 PHOTO-AÉRIENNE-RÉGIONALE : forêts sombres, terroirs reconnaissables (L, Opus pour la conception)
- Spec. Refaire la carte de couleur à partir d'une **référence réelle et libre de droits** : composites médians Copernicus
  Sentinel-2 (licence ouverte Copernicus, données libres ; pas EOX s2cloudless, non commerciale), saison printemps, reprojetés sur
  la grille du jeu et quantifiés en terroirs. On corrige ensuite pour 1337 : forêt + 30 %, aucune monoculture moderne, marais et
  landes restaurés depuis `wetlands.png` et `agri_landscapes.json`.
- Les forêts deviennent une **masse sombre à lisière nette** (Cassini) : v 0,22-0,30. On y ajoute un liseré d'ombre portée côté
  soleil et la canopée en texture de détail. Elles ne sont plus une mousse claire.
- Identités régionales imposées en données : meseta castillane ocre, landes de Gascogne brun-vert, bruyère écossaise, bocage
  normand à haies sombres, champagne crayeuse claire.
- Critères. Avec un masque de forêt tiré du dump `vegetation` ou du splat : v forêt / v pré ≤ 0,55 sur camp_0200, camp_0600 et
  vis_r3b_loire. Castille h 35-55°. Highlands h 20-50°, s 0,2-0,4. Distance de teinte moyenne entre 5 régions de référence ≥ 15°.
  Un testeur aveugle reconnaît 4 régions sur 5 sans plaques.
- Fichiers : `tools/` (pipeline de carte de couleur), `data/map/` (colormap, terroirs), `terrain/{ground_albedo, ground_look,
  fields_paint}.wesl`, `vegetation/`, ADR. Coût CPU et GPU neutre (la carte est cuite). Données : téléchargement Sentinel-2
  gratuit, rien de payant.

### M3 PARCELLAIRE-PEINT (R3b, motifs typés) : lanières texturées, fin des prismes et des décalques codés (M-L)
- Spec. (a) Lanières à d 20-150 : textures de détail Z-Image tuilables (sillons, chaume, jachère, vigne, prairie de fauche ; 8
  cultures × 2 saisons), bord de parcelle irrégulier, tournières et haies en liseré sombre. Le motif typé de R3b (openfield en
  lanières, bocage en mailles, Midi en damier irrégulier) est choisi par terroir. (b) Les modèles de champ glb (`dn_fields.json`)
  sont retirés de la carte de campagne au-delà de 15 m de caméra : ce sont des prismes. Les remplacer par la texture de sol. (c)
  Le décalque `hamlet_core` passe à des images Z-Image (4 palettes, vue zénithale de hameau : jardins, vergers, cour de terre).
  Supprimer `core_pixels` et toute génération d'image au chargement.
- Critères. À d 25-60 : énergie de gradient à l'intérieur d'une parcelle (2-6 px) ≥ 2 × l'actuelle (mesure scriptée sur
  camp_0060). Aucun bord rectiligne > 200 px sans irrégularité. Zéro instance de champ glb à d ≥ 15 (dump `countryside`). Test
  d'inventaire : aucun `Image::new` de contenu visuel dans `campaign/countryside/`. Décalque de hameau visible à d 60-200 (capture).
- Fichiers : `terrain/{parcels, fields_paint}.wesl`, `countryside/{hamlet_core, field_plan, layer}.rs`, `data/art/
  {dn_fields.json, tx_campaign_parcels_pack.json}`, `data/map/map_countryside.json`. Assets : Z-Image local, environ 22 images, aucun
  Meshy.

### M4 HORIZON-ET-ÉCHELLE : fin du L3 et échelle des armées de campagne (S-M)
- Spec. Masquer les étiquettes de régions, fleuves et pays, les plaques et les bannières d'armées dont le point d'ancrage est
  au-delà de la ligne d'horizon ou dont la brume (`DepthFog::amount`) dépasse 0,6. Exposer `Atmosphere.fog` aux `markers/`, comme
  le demandait l'ADR 0833. Échelle des figures d'armée et des chariots plafonnée par rapport aux maquettes de ville. Ciel de
  campagne : saturation de bande haute relevée et nuages lisibles à d 25-120 (couverture visible par temps clair).
- Critères. Zéro étiquette ou plaque projetée au-dessus de l'horizon (section de dump `campaign_labels.above_horizon = 0`,
  vérifiée sur alpes120 et camp_0025). Hauteur du cavalier d'armée ≤ 0,25 × celle de Notre-Dame à d 25. Bande haute du ciel :
  s ≥ 0,25 sur 5 vues sur 5. Au moins 8 % du ciel couvert de nuages visibles par temps clair.
- Fichiers : `client/src/campaign/{markers, armies, weather/dome.rs}`, `data/fx/campaign_sky.json`, `data/ui/campaign_map.json`.
  GPU nul.

### M5 BATAILLE-STABLE-ET-NATURELLE (M) — étape a en urgence
- Spec. (a) **Diagnostic de la postérisation (G1-5).** Rejouer `mb.txt` (fichier de script donné en fin de page) avec
  `CENT_ANS_UNITS_SPEED=16,1`, une capture toutes les 30 images. Couper un à un le sang, la fumée, la poussière, le SSAO et les
  morts (interrupteurs dev) jusqu'à trouver la cause. Ajouter un garde NaN (clamp et `select(isnan)`) dans le shader fautif, et un
  test de capture : 10 images en mêlée, zéro image à plus de 10 % de noir pur. (b) Identifier et corriger les lignes droites de
  G2-4. (c) Ciel de bataille ramené à la bible (v ≥ 0,7, s 0,25-0,35, contraste des nuages réduit). Sol lointain moins délavé
  (s ≥ 0,30, v ≤ 0,60). (d) Teinte héraldique : part des pixels d'étoffe noble à s ≥ 0,35 de 60 % au moins. Le critère porte sur
  les pixels d'étoffe (masque par matériau dans un dump ou un rendu d'identifiants), pas sur la couleur moyenne qui mêle acier et
  robes.
- Critères. Ceux des points a à d. Le banc de mêlée confirme `contact: true` (voir la section sur les triangles).
- Fichiers : `client/src/battle/{units/skinned.wesl, units/corpses.rs, fx/*, decor/ground.wesl, scene/sky*}`,
  `data/fx/{battle_lighting.json, battle_ground.json, battle_soldier_look.json}`, `client/src/agent/bench_battle.rs`.

### M6 JOUR, SAISON ET PARCHEMIN QUI MONTE (R8, heure du jour) (M)
- Spec. (a) R8 : la bascule vers le parchemin ne se fait plus d'un coup. Le parchemin « monte » par-dessus la 3D par l'altitude de
  caméra : il recouvre d'abord la mer et les lointains (masque de profondeur), puis la terre, sur une plage de d 700 à 1 400, avec
  un fondu au grain de papier. (b) Heure du jour en campagne : le tour se joue à midi, mais l'arrivée de tour et la fin de saison
  montrent une lumière d'aube ou de crépuscule (bible : aube 8°, 3 100 K ; crépuscule 9°, 3 300 K) pendant la relecture des
  mouvements de l'IA. La nuit sert aux sièges et aux marches forcées.
- Critères. Aucune image de transition avec plus de 30 % de pixels ni 3D ni parchemin (mesure sur 20 images d'une descente
  scriptée). Aube et crépuscule distincts du midi : Δh du sol ≥ 10° et Δv ≥ 0,1 sur camp_0200.
- Fichiers : `client/src/campaign/{parchment, terrain/post.wesl, weather/atmosphere.rs, camera/zoom.rs}`,
  `data/fx/{campaign_sky.json, atmosphere.json}`.

### Critères ajoutés aux lots en cours
- **VIS-L5 parchemin** : mesurer **après** M1, ou couper la saturation d'écran en vue parchemin dans le lot même. Sinon, aucun
  réglage de lavis ne tient (constat : fonds orange à s 0,35 et frontières fluo sous ×2,8).
- **VIS-L4 eau** : la côte ne doit pas virer au cyan au contact de la terre (h de la frange terre-mer ≤ 190° sur 10 px). Mer large
  à s ≥ 0,55 mesurée sans saturation d'écran supérieure à 1,15 (après M1), sinon le critère est atteint par le mauvais levier.

## Ce que le prochain juge doit vérifier en premier
1. **Postérisation en bataille (G1-5)** : relancer la mêlée avec 10 captures sur 3 minutes ; une seule image postérisée suffit
   pour bloquer.
2. `vis_r4_alpes400` et `camp_1400` : neige blanche (s ≤ 0,10) et parchemin sans fluo, une fois le gain d'écran ramené à
   ≤ 1,15.
3. `camp_0200` avec un masque de forêt : forêts plus sombres que les prés (rapport ≤ 0,55).
4. `vis_r4_alpes120` et `camp_0025` : aucune étiquette dans le ciel.
5. Le banc de mêlée : `contact: true` et triangles revenus vers 6,3 M avant de recalibrer.

Script de mêlée utilisé (dans `out/`, non commité) : `CENT_ANS_UNITS_SPEED=16,1 CENT_ANS_BATTLE_MELEE=28 client/run_bg.sh
--state Battle --offscreen --script mb.txt`, avec `mb.txt` = `wait 2300`, `shot a`, `wait 60`, `shot b`, `wait 60`, `dump`,
`shot c`, `exit`.
