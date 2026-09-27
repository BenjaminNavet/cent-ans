# 0096 — Animation vivante (lots AN1)

Date : 2026-09-27. Statut : proposé (complété au fil des lots).
Orchestration : `docs/wip/an1-animation-vivante.md`.

## A. Mouvement secondaire en shader (AN1a)

(Rédigé par le lot AN1a.)

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
- Changements de clip d'un cycle de mêlée sans fondu (comme avant) ; `idle_helm` lève aussi le
  bouclier au bras gauche (tenu à côté du casque).
- Porte-étendards, musiciens et équipages de siège : pas de nouveaux clips.
