# 0096 — Animation vivante (lots AN1)

Date : 2026-09-27. Statut : proposé (complété au fil des lots).
Orchestration : `docs/wip/an1-animation-vivante.md`.

## A. Mouvement secondaire en shader (AN1a)

### Décision
Un mouvement secondaire bon marché est ajouté **dans le shader de sommets**, après le skinning,
dans le repère posé de la figurine (+Z = avant) : aucun os, aucune simulation CPU, **aucune
recuisson** des figurines.

- **Pièces souples reconnues avec les données déjà cuites** (analyse des maillages fins) :
  - bas des surcots, jaques et cottes (codes livrée, étoffe, gambison) : poids = part des os
    des jambes du sommet (0 à la taille, ~0,85 à l'ourlet, gradient du transfert de poids) ; les
    chausses, entièrement sur les jambes (≥ 0,99), restent rigides ;
  - caparaçon : code armoiries porté par les os du cheval (`sever_bones[0]`), poids selon la
    hauteur de repos (ourlet 0,48 m → selle 1,45 m, données) ;
  - queue : os `Tail1-7`, poids selon le rang dans la chaîne ;
  - crinière : robe du cheval à la teinte exacte des crins (`HAIR_RGB` 0,33 → 84/255 en couleur
    de sommet 8 bits).
- **Moteurs** : vitesse du régiment (`BattleSoldiers._speed`, uniforme `move_speed` envoyé
  seulement quand il change, arrondi à 0,1 m/s) → recul vers l'arrière ; vent de la bataille
  (direction et force de `data/fx/battle_finish.json` `wind`, le même que l'herbe et les
  drapeaux) et rafales ; flottement (deux sinus, phase selon la position de repos et le soldat).
- **Pas de pénétration** : la poussée est pondérée par l'orientation de la face (les faces qui
  regardent la poussée s'évasent, celles qui lui font face restent plaquées sur les jambes ou
  les flancs) ; un « évasement » soulève le bord libre.
- **Étendards** (`battle_standard_flag.gdshader`) : amplitude et vitesse de l'onde lues dans les
  données, ondulation le long de la hampe, et **vent apparent** de l'allure du porteur (clip du
  jeu : arrêt, marche, course, sonnerie → l'étoffe traîne vers l'arrière quand il court).
- **Données** : `data/fx/atmosphere.json` `secondary_motion` (schéma
  `fx_atmosphere.schema.json`) : amplitudes par pièce (recul, vent, flottement, fréquence,
  évasement), échelle du vent, distance maximale (100 m, fondu dès 70 %), vitesse de
  référence. Uniformes posés par `game/scripts/battle/battle_secondary_motion.gd`.
- **Portée** : LOD0/LOD1/LOD2 fins et figurines Quaternius (`--coarse-figures`, mêmes rigs et
  codes) ; figurines rigides `--legacy-figures` inchangées (autre shader) ; éteint sur les
  couches de poses figées (cadavres, blessés, renversés, porte-étendard tombé). `--no-an1a`
  coupe tout (banc A/B).

### Alternatives écartées
- **Canal de poids cuit** (`battle_fine.py`) : plus précis (vraie distance à l'attache), mais
  recuisson complète ~45 min et conflit avec AN1b (textures d'os) ; les poids déduits suffisent.
- **Os de tissu dans les clips** : coût de cuisson et de texture d'os, mouvement répétitif lié au
  clip, pas de vent.
- **Simulation de tissu** (Godot SoftBody) : incompatible avec le rendu par MultiMesh de milliers
  de soldats.

### Conséquences
- Coût GPU au banc FG5 (Ultra, passes alternées avec `--no-an1a`) : standard non mesurable
  (-1 % en médiane), rapproché +1 % en médiane (+2,7 % au minimum). La première version
  (boucles par os, 160 m) coûtait +10 % en rapproché : poids vectorisés, portée ramenée à
  100 m (au-delà, 1 px ≈ 8 cm). Détail : `docs/wip/an1a-mouvement-secondaire.md`.
- ~7 sommets du corps du cheval fin partagent la teinte des crins et bougent avec la crinière
  (quelques millimètres) : invisible ; à retirer si une recuisson ajoute un vrai masque.
- Le mouvement utilise `TIME` (comme les drapeaux) : il continue pendant la pause.
- La vitesse est celle du régiment, pas du soldat (soldats qui se replacent : pas de recul).

## B. Nouveaux clips cuits et branchement (AN1b)

### Contexte
~45 clips cuits (ADR 0014) : pas de victoire, une seule attente par arme, mêlée à trois
gestes, un seul impact, aucun cheval qui refuse les piques ou qui trébuche. Le shader tirait
au plus 4 clips par jeu dans une table de 48.

### Décision
1. **Source : poses calculées par clés dans Blender** (`battle_skinned_poses.py`), sur les
   actions Quaternius CC0 du rig `human` et du cheval, comme les clips existants ; aucune capture
   Mixamo (licence). `hit_b` reprend l'action Quaternius `HitRecieve_2`, inutilisée jusque-là.
   Les clips sont ajoutés **en fin** de `human_clip_specs` et de `battle_skinned_cavalry.clip_specs` :
   les images des anciens clips restent identiques (vérifié octet par octet).
2. **Clips humains (13)** : `victory`, `victory_b` (épée, arc, arbalète), `victory_pike` (armes
   d'hast) ; attentes `idle_look`, `idle_lean` (appuyé sur l'arme), `idle_helm` (ajuste le casque),
   `pike_look`, `bow_look`, `xbow_look` ; `parry` (bouclier levé), `overhead` (taille par-dessus,
   à deux mains) ; impacts `hit_b`, `hit_c` (recul d'un pas). Boucles de 41 ou 82 images (une ou
   deux boucles d'`Idle`) pour que la source boucle avec le clip.
3. **Clips cheval (3)** : `c_rear` (32 images, tient dans le cycle de mêlée de 1,4 s), `c_stumble`
   (90 images = six foulées de galop), `c_victory` (boucle sur `Idle_2`). Le cheval reçoit ses
   propres surcharges (`pose.horse(harm, t)`, avant d'asseoir le cavalier) : tronc pivoté autour
   des hanches, postérieurs replantés par CCD sagittal, sabots (os IK racine) portés par leur
   canon.
4. **Shader** : `clips[64]` (au lieu de 48, aussi dans `battle_standard_flag.gdshader`) et jeux
   jusqu'à **8 clips** : deux indices par composante de `clip_set`/`prev_set` (octet bas =
   emplacements 0-3, octet haut = 4-7, `pick_clip`). Jusqu'à 4 clips le codage est identique à
   l'ancien. Seule la fonction `choose` change ; la partie sommets (AN1a) n'est pas touchée.
5. **Branchement (rendu seulement, aucune règle dans `core/`)** : table `STYLES` de
   `BattleSkinned` (déjà en GDScript, pas en données) enrichie ; états de rendu `victory` (camp
   vainqueur de `get_outcome` une fois `is_finished` ; les autres régiments restent figés comme
   avant) et `melee_pikes` (cavaliers en mêlée dont la cible est un régiment de style `pike`/
   `militia`) ; la charge des lanciers devient un cycle de 3,75 s dont un tirage sur huit est un
   trébuchement (cadence de galop conservée). Clips absents du rig (kit grossier
   `--coarse-figures`, non recuit) écartés du jeu, repli sur `idle`.
6. **Écran de fin retardé de 3 s** (`VICTORY_HOLD`) pour voir l'acclamation ; aucun délai sans
   affichage ni en banc d'essai (tests inchangés).

### Coût
Recuisson des seules textures d'os (`battle_fine.py -- rigs`, 9 s ; les maillages ne dépendent
pas des clips). `human.bones.bin` 1,49 → 2,01 Mo (2 101 → 2 843 images, 3,4 Mo en VRAM),
`cavalry.bones.bin` 1,75 → 2,26 Mo (781 → 984 images, 3,3 Mo en VRAM).

### Limites
- Kit grossier Quaternius non recuit : il garde les anciens jeux.
- ~~Changements de clip d'un cycle de mêlée sans fondu~~ : fondu de 0,2 s depuis NT7
  (ADR 0129). `idle_helm` lève aussi le bouclier au bras gauche (tenu à côté du casque).
- ~~Porte-étendards, musiciens et équipages de siège : pas de nouveaux clips~~ : clips propres
  de charge, victoire, attente et gestes de servants depuis NT7 (ADR 0129). La table `clips[]`
  passe de 64 à 96 (rig humain fin : 70 clips).
