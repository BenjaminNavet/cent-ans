# Audit A1 — visuel, 3D, animation (session de nuit 7)

Date : 2026-09-24. Auditeur : A1 (lecture seule, aucun fichier du jeu modifié).
Référence : Total War récents (Medieval II pour l'époque, Three Kingdoms / Pharaoh / Warhammer III
pour la finition). Direction artistique : ADR 0004 (semi-réaliste, lumière d'abord, PBR, parchemin pour l'UI).
Build testé : `main` au commit `70741c1`, dylib debug du 24/09 22:45, Godot 4.7.2, Forward+, Mac de dev.

## 1. Captures

Toutes dans `docs/audit/captures/a1/` (fenêtre 1920×1080 demandée, rendu effectif 1440×900 HiDPI).

| Fichier | Contenu | Commande |
|---|---|---|
| `01_menu.png` | Menu d'accueil | `godot --path game res://scenes/start_menu.tscn -- --screenshot=<png>` |
| `02_campagne_defaut.png`, `03_campagne_armee.png` | Carte, cadrage par défaut ; armée sélectionnée + chemin | `res://scenes/campaign_map.tscn -- [--stage=map] --screenshot=<png>` |
| `04_campagne_tres_loin.png` … `07_campagne_tres_proche_paris.png` | Zooms 1500 / 450 / 70 / 25 sur Paris | `… --stage=map --focus=2213,1924,<dist>` |
| `08_campagne_alpes.png`, `09_campagne_cote_calais.png` | Relief alpin ; côte et mer | `--focus=2548,2461,120` ; `--focus=2198,1596,45` |
| `10_fiche_settlement.png`, `10_fiche_city.png`, `10_fiche_province.png` | Fiche de colonie (La Rochelle), fiche de province | `--stage=settlement|city|province` |
| `11_bataille_deploiement.png` | Déploiement | `res://scenes/battle/battle.tscn -- --deploy-shot --screenshot=<png>` |
| `12_bataille_melee.png`, `22_bataille_melee_gue.png`, `23_bataille_melee_rapprochee.png` | Mêlée (caméra auto, puis `--camera=x,z,dist,yaw`) | `… --screenshot=<png> [--camera=600,400,90,35]` |
| `13_bataille_melee_gros_plan.png`, `16_bataille_pluie.png` | Gros plan (cavalerie), pluie | `--closeup`, `--weather=rain` |
| `14_bataille_siege.png` | Assaut de Guyenne | `--siege` |
| `15_bataille_fin.png` | Écran de fin de bataille | `--result-shot` |
| `17_incendie_proche.png`, `18_incendie_large.png`, `19_ruines.png` | Incendie S2 | `--script res://tests/s2_fire_shot.gd -- --out=<dossier>` |
| `20_effondrement_muraille.png`, `21_breche_apres.png` | Effondrement S1 | `--script res://tests/s1_collapse_shot.gd -- --out=<dossier>` |
| `24_figurines_{melee,charge,archers,volley,bombard}.png` | Figurines et animations en gros plan (banc B4, sol plat de test) | `--script res://tests/b4_figures_shot.gd -- --out=<png> --shot=<x>` |

Écran de fin de campagne : pas de mise en scène de capture ; lu dans le code
(`game/scripts/map/victory_controller.gd` : boîte 560×260, titre, texte, score, deux boutons).

## 2. Évaluation face à un Total War moderne

Note de 0 à 10 (10 = niveau TW récent). Le jeu a beaucoup progressé depuis l'ADR 0004 ; les notes
jugent l'image, pas la simulation.

| Domaine | Note | Constat |
|---|---|---|
| Relief de campagne | 6 | Alpes réussies (exagération verticale, neige, conifères, `08`). Plaines crédibles au zoom proche (parcellaire, `06`/`07`). Au zoom moyen (`05`) la plaine est une bouillie olive-grise uniforme avec une grande tache sombre (ombre/forêt) sans lecture. |
| Eau de campagne | 4 | Mer turquoise plate, sans reflet solaire ni houle lisible, côte sans plage ni falaise ni ressac (`09`). Les étangs ont un **halo blanc** d'écume en anneau (`09`, artefact). Les fleuves sont un **ruban bleu translucide posé sur les champs** qui traverse la muraille de Paris sans pont (`07`). |
| Végétation de campagne | 4 | Arbres « sucettes » tous identiques, isolés, sans masse forestière (`06`, `07`, `09`) ; seules les forêts de montagne ont de la densité (`08`). TW3K/Pharaoh : forêts en canopée continue, essences variées. |
| Colonies sur la carte | 4 | Maquettes low-poly sans texture (toits aplats, murs gris), échelle jouet ; Paris sans Seine intégrée, sans pont, sans faubourgs (`07`). Châteaux lisibles (Vincennes). |
| Lisibilité de campagne | 5 | Brouillard de guerre qui **noircit** l'Angleterre (`02`, `05`) : on croit à un bug de rendu. Trop de pictogrammes au zoom moyen (points, étoiles, écus, anneaux verts `03`). Étiquettes à contour épais, pas d'étiquettes de province courbes façon TW au dézoom (`04`). |
| Ciel et lumière | 6 | AgX, SSAO, ombres douces 8192, brouillard de distance et DOF lointain corrects. Pas de SSIL/SDFGI (pas de lumière rebondie : ombres propres mais scène « plastique »), pas de brouillard volumétrique, ciel de bataille en dégradé simple, pas d'heure du jour ni d'étalonnage par saison. |
| Terrain de bataille | 4 | Sol splat PBR correct de près, mais vue de jeu (`11`, `12`) = plaine uniforme jaune-vert sans relief ni accident ; la **zone de déploiement recouvre tout l'écran en jaune** (`11`). Rivière = plan bleu clair à bords nets, les soldats marchent dessus sans s'y enfoncer ni éclabousser (`22`, `23`). |
| Soldats (maillages) | 2 | Écart n° 1, inchangé depuis le constat du plan TW : **mannequins à membres tubulaires**, bras « saucisses », visages sans traits, couleurs unies sans texture (pas de maille, de tissu, de cuir) (`24_melee`, `24_archers`). 3 variantes par classe seulement ; à distance de jeu, les régiments sont des traits de points (`11`, `12`). |
| Chevaux | 3 | Silhouette et caparaçon fleurdelisé réussis (`24_charge`) mais jambes en allumettes, têtes illisibles à distance, caparaçon bleu qui fond cheval et cavalier dans une même masse (`13`). |
| Animations | 3 | Animation par shader de membres rigides (`battle_soldier.gdshader`) : marche et galop existent, mais pas de squelette ni de déformation, pas de duels appariés, pas de réaction aux coups, pas d'impact de charge, chutes raides. L'archer ne tire pas la corde à la joue (`24_archers`). |
| Effets | 4 | Poussière de charge et traînées de flèches correctes (`24_charge`, `24_volley`). Flammes = taches floues sans texture animée, sans éclat sur les murs (`17`) ; fumée de bombarde = sphère grise (`24_bombard`) ; flèches plantées = confettis blancs trop gros et trop clairs (`14`) ; pas de sang (choix assumé, cf. `battle_effects.gd`). |
| Bâtiments de bataille / siège | 5 | Murailles texturées pierre, tours coniques, échelles : bon niveau (`14`). Mais intérieur de ville vide (disque « place » au milieu, maisons clairsemées, `18`) ; église en **blanc sans texture** (`17`, `19`) ; maisons brûlées = dalles noires avec toit rouge posé à plat (`19`). |
| Effondrement de muraille | 5 | Physique réelle et poussière convaincantes (`20`) mais blocs géants réguliers, texture étirée : effet « dominos ». |
| Caméra | 5 | Campagne : bonne inclinaison liée au zoom. Bataille : vue par défaut trop haute (soldats = points) ; rapprochée, l'inclinaison reste rasante (`23`) ; pas de mode suivi (lot T6). |
| Écrans de fin | 3 | Fin de bataille : tableau de chiffres sur parchemin, sans illustration, grand vide central (`15`). Fin de campagne : boîte de texte 560×260. TW : peinture plein écran, héros de la bataille, graphiques. |
| Menu d'accueil | 6 | Belle carte parchemin, cohérente ; statique ; blasons dessinés grossièrement (léopards d'Angleterre en tache, `01`). |
| Cohérence DA | 5 | L'UI parchemin est homogène. La 3D mêle PBR (murailles, sol, relief) et aplats non texturés (soldats, église, maquettes de colonies) : c'est cette rupture qui fait « prototype ». |

Synthèse : décor (relief, murailles, lumière) à mi-chemin d'un TW ; **tout ce qui est vivant (soldats,
chevaux, animations, feu) reste au niveau prototype**, et c'est ce que le joueur regarde en bataille.

## 3. Lots d'amélioration

Effort : S (≤ ½ j agent), M (1-2 j), L (≥ 3 j). Impact 1-5. Ratio = impact / effort (S=1, M=2, L=3).
Aucun lot ne touche `core/` sauf mention ; tous sont rendu (`game/`) + outils (`tools/`).

### 3.1 Classement par ratio impact/effort

| # | Lot | Impact | Effort | Ratio |
|---|---|---|---|---|
| A1-01 | Lisibilité des régiments à distance | 4 | S | 4,0 |
| A1-02 | Zone de déploiement discrète | 3 | S | 3,0 |
| A1-03 | Brouillard de guerre non noir | 3 | S | 3,0 |
| A1-04 | Halo des étangs et côtes | 3 | S | 3,0 |
| A1-05 | Ciels HDRI et étalonnage par météo/saison | 3 | S | 3,0 |
| A1-06 | Caméra de bataille à la TW | 3 | S | 3,0 |
| A1-07 | Textures et matières des soldats (sur maillages actuels) | 4 | M | 2,0 |
| A1-08 | Variété des soldats | 4 | M | 2,0 |
| A1-09 | Champ de bataille vivant (relief, accidents, décor) | 4 | M | 2,0 |
| A1-10 | Forêts et essences de la carte | 4 | M | 2,0 |
| A1-11 | Fleuves et ponts de campagne | 4 | M | 2,0 |
| A1-12 | Villes de siège denses et texturées | 4 | M | 2,0 |
| A1-13 | Feu, fumée, braises en flipbook | 3 | S-M | 2,0 |
| A1-14 | Lumière rebondie et brume (SSIL/volumétrique) | 3 | S-M | 2,0 |
| A1-15 | Écrans de fin illustrés | 3 | S-M | 2,0 |
| A1-16 | Flèches plantées, bombarde, petits effets | 2 | S | 2,0 |
| A1-17 | Désencombrement des pictogrammes de campagne | 4 | M | 2,0 |
| A1-18 | Soldats et chevaux skinnés + animations VAT | 5 | L | 1,7 |
| A1-19 | Colonies de campagne texturées et monuments | 4 | M-L | 1,6 |
| A1-20 | Combat apparié, impacts de charge, morts | 4 | M-L | 1,6 |
| A1-21 | Rivière de bataille (gué, berges, éclaboussures) | 3 | M | 1,5 |
| A1-22 | Mer et côtes | 3 | M | 1,5 |
| A1-23 | Effondrement fracturé | 3 | M | 1,5 |
| A1-24 | Menu animé et héraldique redessinée | 2 | S-M | 1,3 |

### 3.2 Fiches

**A1-01 Lisibilité des régiments à distance** — `game/shaders/battle_soldier.gdshader`, `game/scripts/battle/battle_soldiers.gd`.
Au-delà de ~80 m, teinte de livrée renforcée et légère émission/rim-light de camp (comme TW qui « colore » les
unités au loin), échelle ×1,15 des figurines `far`, contour de sélection. Shader uniquement. Pas de dépendance.

**A1-02 Zone de déploiement discrète** — `game/scripts/battle/deployment_zone.gd`.
Remplacer l'aplat jaune plein écran par un liseré lumineux + hachures faibles en bordure (Warhammer III), fondu
avec la distance caméra. S.

**A1-03 Brouillard de guerre non noir** — `game/shaders/terrain.gdshader` (`fog_mask`), `game/shaders/campaign_minimap.gdshader`.
Désaturer + voile parchemin/brume clair au lieu d'assombrir ; option : nuages bas animés (TW3K). S. Dépend du lot C1.

**A1-04 Halo des étangs et côtes** — `game/shaders/water.gdshader`, `game/scripts/map/sea.gd`, `coast_renderer.gd`.
L'écume en anneau blanc autour des petits plans d'eau vient du seuil `coast_dist` ; la limiter à la mer, fondre la
berge (boue, roseaux) pour les lacs. S.

**A1-05 Ciels HDRI et étalonnage** — `game/shaders/battle_sky.gdshader`, `game/scripts/battle/battle_atmosphere.gd`,
`game/scripts/visual/campaign_atmosphere.gd`. HDRI CC0 Poly Haven (clair, couvert, pluie, crépuscule) en
`PanoramaSkyMaterial` ; soleil orienté selon l'heure ; LUT d'étalonnage par saison/météo (printemps vert, automne
doré, hiver froid). 0 $. S.

**A1-06 Caméra de bataille** — `game/scripts/battle/battle_camera.gd`. Inclinaison liée au zoom (plongée au loin,
presque à hauteur d'homme près du sol, comme la carte), hauteur minimale suivant le terrain, vue par défaut plus
basse et plus proche (on doit voir des hommes, pas des points), mode suivi de régiment (T6). S.

**A1-07 Textures et matières des soldats** — `game/shaders/battle_soldier.gdshader`, `tools/blender_scripts/` (figures B1),
`game/assets/textures/battle/`. Sans changer les maillages : triplanaire de maille (normal map annelée), tissu tissé,
cuir, métal brossé avec rugosité variable ; visage peint (yeux, barbe) par une petite texture atlas générée
(OpenRouter ≈ 0,2 $ ou peinte procéduralement). Gain immédiat de « sérieux ». M. Réutilisable par A1-18.

**A1-08 Variété des soldats** — `game/scripts/battle/battle_meshes.gd`, `game/assets/models/battle/figures.json`.
6-8 variantes par classe (bassinet à camail, chapeau de fer, cervellière, capuchon, brigandine, gambison), taille et
corpulence aléatoires via `INSTANCE_CUSTOM`, teintes de tissus par soldat, pennons et bannières de seigneurs par
régiment (héraldique `data/`). M. Dépend de rien (mieux après A1-18 si fait ensemble).

**A1-09 Champ de bataille vivant** — `game/scripts/battle/battle_terrain.gd`, `battle_vegetation.gd`, `game/shaders/battle_ground.gdshader`.
Relief réel tiré de la heightmap de campagne à l'endroit de la bataille (ondulations, crêtes : Crécy se joue sur une
pente), variation de couleur macro, champs cultivés, haies, murets, rochers, chemins creusés d'ornières, bosquets
denses en lisière. Assets Poly Haven CC0 / Blender procédural. M.

**A1-10 Forêts et essences de campagne** — `game/scripts/map/vegetation.gd`, `game/shaders/foliage.gdshader`.
3-4 essences (chêne, hêtre, peuplier, pin) en cartes croisées/imposteurs octaédriques cuits sous Blender ; forêts en
canopée continue (masse + arbres de lisière) selon la splatmap ; teinte saisonnière. M.

**A1-11 Fleuves et ponts de campagne** — `game/scripts/map/rivers_renderer.gd`, `game/shaders/water.gdshader`, `data/map/crossings.json`.
Lit creusé dans le terrain, eau avec profondeur, écoulement et reflets (mêmes paramètres que la mer), berges ; ponts
de pierre/bois aux franchissements déjà connus (`crossings.json`) ; la Seine passe *sous* Paris par des ponts. M.

**A1-12 Villes de siège denses et texturées** — `game/scripts/battle/battle_village.gd`, `battle_siege.gd`, `battle_siege_batcher.gd`.
Tracé de rues, îlots serrés, place de marché avec étals, puits, charrettes ; église et donjon texturés (les textures
Poly Haven `plastered_wall`, `roof_slates` sont déjà dans `game/assets/textures/battle/`) ; ruines = murs calcinés
à trous + charpente noircie au lieu de dalles noires. M.

**A1-13 Feu, fumée, braises** — `game/scripts/battle/siege_fire_fx.gd`, `battle_effects.gd`.
Flipbooks de flammes et de fumée (simulation Blender Mantaflow cuite en planches 8×8, 0 $), braises, lumière
vacillante sur les façades, matériau qui noircit progressivement ; fumée de bombarde en panache + éclair de bouche. S-M.

**A1-14 Lumière rebondie et brume** — `game/scenes/battle/battle.tscn`, `game/scripts/visual/`.
SSIL (qualité moyenne) en bataille, brouillard volumétrique bas au matin/sous la pluie, ombres de nuages projetées
(texture de bruit défilante sur la carte et en bataille). Préréglage « qualité » dans les réglages (conforme ADR 0004). S-M.

**A1-15 Écrans de fin illustrés** — `game/scripts/battle/battle_result_screen.gd`, `game/scripts/map/victory_controller.gd`.
Peinture plein écran par issue et faction (victoire/défaite × 3 factions ; OpenRouter ≈ 0,5 $, style enluminure),
régiment héroïque, graphique de pertes, bannières capturées ; fin de campagne en page de chronique enluminée. S-M.
Coordonner avec l'auditeur UI.

**A1-16 Petits effets** — `game/scripts/battle/battle_effects.gd`, `battle_siege.gd`.
Flèches plantées plus sombres et plus fines, disparition progressive ; bombarde d'époque (pot de fer / bombarde cerclée
sur affût de bois, Blender) ; option « sang » désactivée par défaut (taches au sol, pas de gerbes). S.

**A1-17 Désencombrement de campagne** — `game/scripts/map/city_markers.gd`, `settlement_layer.gd`, étiquettes.
Hiérarchie par zoom : au loin, seulement noms de provinces en lettres espacées courbées selon la forme (TW) ; au
moyen, villes principales ; au proche, tout. Anneaux verts de sélection plus discrets. M. Coordonner avec l'auditeur UI.

**A1-18 Soldats et chevaux skinnés + animations VAT** — `tools/blender_scripts/`, `game/assets/models/battle/`,
`game/shaders/battle_soldier.gdshader`, `game/scripts/battle/battle_meshes.gd`, `battle_soldiers.gd`.
C'est le chantier décisif. Corps humain continu (base CC0 MakeHuman ou modélisée sous Blender, 3-5 k triangles
proches, LOD 800/200), armures et vêtements en pièces, squelette ; animations clés (repos, marche, course, frappes ×3,
parade, tir à l'arc complet, recharge d'arbalète, chutes ×3, fuite, galop, cabré) **cuites en textures d'animation
(Vertex Animation Textures)** lues dans le shader : on garde le `MultiMesh` et les milliers de soldats. Cheval :
maillage anatomique (encolure, tête, sabots), 3 robes, bardes variées. L. Dépendance : A1-07/A1-08 s'y intègrent.

**A1-19 Colonies de campagne et monuments** — `game/assets/models/settlements/`, `tools/blender_scripts/`, `game/scripts/map/settlement_layer.gd`.
Textures (atlas tuiles/ardoise/chaume, pierre, colombage) sur les maquettes, faubourgs et champs autour des villes,
monuments reconnaissables (Notre-Dame, Louvre de Charles V, Tour de Londres, palais des Papes) en maquettes Blender. M-L.

**A1-20 Combat apparié, impacts, morts** — `game/scripts/battle/battle_soldiers.gd`, shader ; lecture seule de `sim-battle`
(positions déjà fournies ; si un appariement est nécessaire, petit ajout de getter dans `core/crates/godot-bridge`).
Paires attaquant/défenseur synchronisées, recul au coup, cavaliers qui bousculent et renversent à l'impact de charge
(TW : « impact »), morts variées tombant dans la direction du coup. M-L. Dépend de A1-18.

**A1-21 Rivière de bataille** — `game/shaders/battle_water.gdshader`, `battle_terrain.gd`, `battle_soldier.gdshader`.
Berges en pente, couleur selon la profondeur, soldats enfoncés à mi-jambe au gué (offset vertical par la profondeur),
éclaboussures et sillage (pool `_splash` déjà présent dans `battle_effects.gd`). M.

**A1-22 Mer et côtes** — `game/shaders/water.gdshader`, `coast_renderer.gd`, splatmap (`tools/cent_ans_tools/geo/`).
Reflet solaire, houle à plusieurs octaves, ressac animé, plages de sable et falaises selon la pente côtière. M.

**A1-23 Effondrement fracturé** — `game/scripts/battle/wall_collapse_fx.gd`, `tools/blender_scripts/`.
Pans pré-fracturés (Cell Fracture de Blender, fragments irréguliers de tailles variées) + gravats fins et poussière
persistante ; UV en coordonnées monde pour éviter la texture étirée. M.

**A1-24 Menu et héraldique** — `game/scenes/start_menu.tscn`, `game/assets/heraldry/`.
Blasons redessinés proprement (SVG vectoriels, léopards lisibles ; Angleterre 1337 = trois léopards, écartelé de
France seulement à partir de 1340), fond de menu légèrement animé (carte qui se déroule, bougie, ou vue 3D de la
campagne en arrière-plan). S-M.

## 4. Ordre recommandé (10 lots prioritaires)

Le ratio seul sous-estime A1-18 : sans nouveaux soldats, les autres lots de bataille plafonnent. Ordre proposé :

1. **A1-18** soldats/chevaux skinnés + VAT (L, impact 5) — lancer tôt, c'est le chemin critique.
2. **A1-07** textures et matières des soldats (M, 4) — gain visible dès demain, réutilisé par A1-18.
3. **A1-01** lisibilité des régiments à distance (S, 4).
4. **A1-06** caméra de bataille (S, 3).
5. **A1-09** champ de bataille vivant (M, 4).
6. **A1-02 + A1-03 + A1-04** correctifs rapides : déploiement, brouillard, halos (S chacun).
7. **A1-10** forêts et essences de campagne (M, 4).
8. **A1-11** fleuves et ponts (M, 4).
9. **A1-12 + A1-13** villes de siège denses + feu en flipbook (M + S-M).
10. **A1-05 + A1-14** ciels HDRI, étalonnage, SSIL/brume (S + S-M).

Budget cloud estimé : ≈ 1 $ (visages A1-07, illustrations A1-15) ; le reste est CC0 ou procédural Blender.
Remarques hors périmètre visuel : en `--weather=rain` la bannière de bataille indique encore « Temps clair »
(forçage de capture seulement ?) ; la barre d'unités occupe ~22 % de la hauteur à 1440×900 (à voir avec A2/UI).
