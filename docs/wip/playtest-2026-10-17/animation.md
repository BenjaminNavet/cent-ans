# Rapport animation (testeur QA)

Dossier de travail : `$P/animation/` (`$P` = /private/tmp/claude-501/-Users-jean-hubert-dev-game-project/a01cdf5d-824f-4e71-9629-e357ad3dd950/scratchpad/playtest).
Aucun `ERROR` / `panicked` / `AGENT_FATAL` dans les journaux des runs (seulement 3 WARN bevy connus : ShadowLodOrigin, prepass normales, winit Destroyed).
Crochets utilisés : `CENT_ANS_NO_DEPLOY=1`, `CENT_ANS_UNITS_SPEED=<k>,1` (IA des deux camps), `CENT_ANS_UNITS_VIEW=<id>,<m>`, `CENT_ANS_BATTLE_MELEE`, `CENT_ANS_CAMERA`, `CENT_ANS_ARMY_AUTOTEST`.

## Défauts (par gravité)

### 1. [majeur visuel] Charrette de campagne : cargaison et bête d'attelage rendues en bouillie noir/blanc déchiquetée
- Repro : `CENT_ANS_NO_BRIEFING=1 CENT_ANS_CAMERA="2.40,48.85,6" $P/play.sh --state Campaign --script $P/animation/cz.txt` (charrette à droite de Paris, près de l'ost).
- Observé : la cargaison (charrette de pierre) est un amas de polygones noirs / blancs sans forme lisible, le bœuf devant est criblé de trous (bords dithérés/éclatés, silhouette illisible). Identique sur les 6 captures (60 images) : la charrette ne bouge pas. Même aspect à distance 60 (`cB_z.png`, charrette en bas à gauche, qui tourne entre les images).
- Attendu : charrette de bois / pierre lisible, bœuf plein, comme les chevaux de bataille.
- Preuve : `animation/cz_cart.png` (recadrages), `cz_0.png`, `am2_0.png`, `cB_z.png`.
- Cause probable : matériau `ExtendedMaterial` à étage de sommets de la vie de la carte (`client/src/campaign/life/anim.rs`, `anim.wesl`, réglage `data/fx/animal_motion.json`) qui déforme ou dithère les maillages `env_cart_*` / `animal_ox_draught` ; à recouper avec les maillages Meshy du lot CV (ADR 0354). Non vérifié dans le code.

### 2. [mineur] Compteurs `campaign_life.live` : `carts: 2` mais `draught_beasts: 0`
- Repro : dump après 200-800 images, `CENT_ANS_CAMERA="0.3,49.0,60"` ou Paris : `live = {carts: 2, draught_beasts: 0}` alors que `draught_beasts` vaut 2 au niveau supérieur (`cp_d1.json`). Visuellement la charrette de Paris a bien une bête. Soit compteur faux, soit attelage non lié au comptage « live ». À vérifier avec l'objectif « pas d'animaux tirés sans être attelés » : non démontré visuellement.

### 3. [cosmétique] Pavois : planches plates unies, sans bois ni peinture, restées sur place
- Repro : bataille perso par défaut, `CENT_ANS_UNITS_SPEED=6,1`, ~900 images (`animation/m1.txt`, `a5.txt`).
- Observé : rangées de rectangles gris-lilas (France) ou saumon (Angleterre) sans texture ; un cadavre traverse un pavois (`t5z.png`, jambes à travers la planche) ; les pavois restent seuls quand les arbalétriers avancent (les régiments du premier plan disparaissent de leur ligne, seules les planches restent : `m1_0.png`, `q_sheet.png`).
- Cause : `client/src/battle/units/volleys.rs` (~550, `_pavise_mesh` 0,6 x 1,15 m, couleur de camp seule).

### 4. [cosmétique] Morts : corps couchés bras écartés (pose en croix), très clairs
- Preuve : `t5z.png` (haut) : cadavres blancs/crème, bras en croix au sol, un corps recouvert d'un caparaçon rouge. Pas de réaction de chute visible sur les captures (clip de chute non vérifié en mouvement).

### 5. [cosmétique] Chevaux de bataille : franges noires dithérées (crins / caparaçons) et plaques grises triangulaires sur les flancs
- Preuve : `t6.png` (cavalerie en mêlée, plaques grises sur le flanc des chevaux bruns), `t5z.png` (franges noires hachées sous les caparaçons). Probable alpha-cutout des crins/housses et décal gris de la figurine cheval (`client/src/battle/units/skinned.rs`, atlas figurines).

### 6. [cosmétique, hors domaine] Déploiement du siège : le discours d'Édouard III recouvre le bouton « Commencer la bataille (Entrée) »
- Repro : `$P/animation/sg.txt` (démo siège de Paris), `animation/sg_sheet.png`.

### 7. [observation] Fin de bataille : états figés
- Après `finished: true`, `soldiers_by_state` reste `{marching 389, routing 686, turn_r 59, victory 1528}` pendant 400 s de sim (dumps `r1..r10`, `t9..t24`) : les fuyards « routing » ne quittent jamais le champ et 389 « marching » marchent sans bouger les dumps. Pas de preuve visuelle que ce soit visible (plan de victoire non testé).

## Ce qui marche bien
- Infanterie à l'arrêt : idle avec décalage de phases (pas de synchro parfaite), livrée et cottes variées, lances droites, pas de T-pose (`i0_cmp.png`).
- Marche d'infanterie (`k0_zoom.png`) : phases de pas différentes par soldat, pas de glissement évident.
- Cavalerie à l'arrêt et au pas (`i3_cmp.png`, `k3_zoom.png`) : pattes arrière/avant déphasées, cavaliers stables, lances levées, pas de flottement. Charge de cavalerie : files régulières, lances baissées, chocs avec états `charging`/`melee` dans le dump (`t_sheet.png`, `t5.png`, `t6.png`).
- Mêlée, déroute (`routing`), piquiers (`melee_pikes`), morts + armes lâchées (`battle_fallen.dropped_arms`), traînées de flèches (`battle_arrow_trails`), volées (`launched` ~7000, `stuck` > 1100), poussière, bannières ondulées (`waved 34/35`).
- Siège de Paris : incendie de maisons + fumée, trébuchet qui cycle (bras -29 deg à 103 deg), servants `push`/`haul` (`sg2_sheet.png`).
- Campagne : villageois qui marchent près de Paris (`cA_sheet.png`), oiseaux en vol, troupeaux animés (phases `a` distinctes dans `campaign_anim.binder.samples`), armée de campagne en figurines skinnées (2 figures pour 620 hommes, lord à cheval avec bannière).

## Non testé / limites
- Pas de mesure quantitative de glissement des pieds (captures à ~4-10 images, caméra souvent éloignée ; régiments en LOD `far` sauf cadrage forcé).
- Course d'infanterie en gros plan, chute individuelle des morts en mouvement, tir à l'arc/arbalète en gros plan (les archers avancent hors cadre ; seules les volées lointaines et les compteurs ont été vus), cavalerie au trot/galop en gros plan (galop vu uniquement de loin).
- Engins de siège : bélier jamais vu en mouvement (`pivot_deg 0`, pas de brèche dans la fenêtre testée), trébuchet de loin seulement.
- Bannières royales / étendards portés : `battle_standards.carried_shown = 0` pendant tous les dumps (non démontré).
- Campagne : moulins en mouvement (1 sail compté, pas regardé en gros plan), eau, nuages, convois en marche, armée en déplacement (autotest lancé mais ost à la ville, glissement non visible), fumée de bivouac.
- Le temps de simulation dépend de la charge machine (même nombre d'images, horloge de bataille très variable) : les fenêtres de mêlée sont difficiles à viser par script ; ne pas se fier au numéro d'image.
