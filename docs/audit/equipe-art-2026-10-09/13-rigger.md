# Rigger — état des lieux (09/10, lecture seule)

## 1. État actuel
- Une seule chaîne, ni `Skeleton3D` ni `AnimationPlayer` en jeu (ADR 0014, 0089, `docs/animation.md`) : squelette Quaternius (CC0) recalé MakeHuman (`battle_fine_rig.py`), clips cuits en texture d'os `CAB1` (3 texels RGBA32F/os/image, une texture par rig pour tous les LOD), maillages `CAM1` 4 os/sommet, `MultiMesh` par régiment et LOD. Shader `battle_soldier_skinned.gdshader` + `battle_bone_anim.gdshaderinc` (`clips[96]`), chargement `battle_skinned.gd` (1 116 l.).
- Rigs (`battle_fine/manifest.json`) : `human` 25 os / 70 clips / 2,4 Mo ; `cavalry` 70 os (cheval + `R:`) / 39 clips / 3,4 Mo ; os virtuels `Prop`, `Nock`, `Arrow`. Copie grossière dans `battle_skinned/`.
- Figurines GA3 (`ga3_figures.py`, 1 882 l.) : TRELLIS A-pose → solidify + remesh voxel → `fit_rig_to_mesh` → bone heat → `strip_arm_leaks` → greffe tête/mains. Cavalier lié aux os `R:`, cheval FG4. 16 figurines (ADR 0140, 0211 D2).
- Retarget : `fa3_anim_retarget.py` (Mesh2Motion, KayKit, IK jambes ; 13 clips dont 3 par défaut), `nt12_mocap_trial.py` (CMU), `nt13_video_trial.py` (MediaPipe) ; chevaux trot/galop Muybridge (AS8b).
- Attaches : arme 2 mains sur `Prop`, 1 main sur poignet, arc sur `Nock`/`Arrow` ; cavalier dans le même rig que le cheval ; `sever_table`.
- Foules : imposteurs (`battle_impostors.gd`), cadavres figés, figurines de campagne (`army_figures.gd`), bêtes en shader (ADR 0188).

## 2. Forces
- Architecture pour des milliers de soldats, fondus de clips dans le shader.
- Cuissons reproductibles (GA3 identique à l'octet).
- Drapeaux A/B partout ; clips choisis sur mesures ; licences propres.

## 3. Faiblesses
1. Mains en moufle, pas d'os de doigts, pas de correction de prise.
2. Une seule variante par figurine GA3.
3. Jupe du jaque liée aux cuisses ; secondaire retiré (ADR 0230) : surcots, caparaçons, crins rigides.
4. Pas du cheval `c_walk` : 49 % de patinage ; `c_std_trot` cuit mais pas branché.
5. Reciblage : contrapposto hérité (NT12 tourné de 27°), piques KayKit genoux rentrés.
6. Pas d'IK de pied sur pente ni de rotation de tête.
7. Pas de cheval GA3 (rig quadrupède).
8. ~6 Mo de textures d'os en double ; marge 26 clips (human), 57 (cavalry).
9. AS7 (jugement en jeu) jamais fait.
10. Selle/étrier vérifiés à la main seulement.

## 4. Améliorations
| # | Action | Impact | Effort | Coût | Dépend de |
|---|---|---|---|---|---|
| 1 | Refaire `c_walk` sur Muybridge, brancher `c_std_trot` | Fort | S | 0 | animateur |
| 2 | Contrôle auto des poids GA3 (fuites, étirement) en pytest | Moyen | S | 0 | tech art, QA |
| 3 | Correction de prise par arme (`Grip.R`) | Moyen-fort | M | 0 | props |
| 4 | Variantes GA3 (têtes, couvre-chefs greffés) | Fort | M | ~0,16 $/var. | 3D, DA |
| 5 | Os de jupe/caparaçon avec ressort cuit (compatible ADR 0230) | Moyen | M | 0 | animateur, DA |
| 6 | Inclinaison/IK selon pente dans le shader | Moyen | M | GPU faible | rendu, terrain |
| 7 | Recuire NT12 axes monde ; piques avec IK genoux | Faible-moyen | S | 0 | animateur |
| 8 | Compression textures d'os (RGBA16F ou quat+trans), retirer le doublon | Faible | M | 0 | moteur |
| 9 | Rig quadrupède auto pour cheval GA3 teintable | Moyen | L | ~0,2 $/essai | 3D, DA |
| 10 | Tête tournée vers l'ennemi (uniforme régiment) | Faible-moyen | S | 0 | gameplay |
| 11 | Session AS7 (virage, trot, selle) | Débloque 1 et 5 | S | 0 | joueur, QA |

Ordre : 11, 1, 2 → 4, 3 → 5, 6.
