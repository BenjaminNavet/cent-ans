# AS — sources d'animation gratuites et licences (08/10)

Recherche web du 08/10, complète `mocap-gratuite.md` (NT12). Points marqués [?] : licence non
relue sur le fichier officiel ; à vérifier avant tout usage par défaut. Pas un avis juridique.

## Retenu / possible
| Famille | Option | Licence | Verdict |
|---|---|---|---|
| Vidéo → humain | MediaPipe Pose | Apache 2.0 (poids Google, model card [?]) | en place (NT13/NT14) |
| Vidéo → humain | RTMPose / RTMW 2D (rtmlib) | code Apache 2.0 ; poids entraînés sur jeux mélangés, certains peut-être recherche seule [?] | essai RT ; pas de défaut sans licence établie |
| Multi-caméra | FreeMoCap (AGPL, outil interne), Pose2Sim (BSD [?]) | outil seulement, animations produites à nous | piste : 2 téléphones = vraie profondeur |
| Bibliothèques | Quaternius UAL 1 et 2, Mesh2Motion | CC0 (Mesh2Motion LICENSE [?]) | prioritaire, déjà base du rig |
| Bibliothèques | CMU | libre, redistribution permise (FAQ CMU, cf. NT12) | en place (NT12) |
| Chevaux | Muybridge (domaine public) comme référence + animation à la main | — | recommandé : aucun modèle ni pack cheval gratuit propre |
| Procédural | shaders (vent, tissus, houle), IK Godot `SkeletonModifier3D`, Jolt, VAT pour foules | MIT | voie par défaut hors personnages |

## Exclus
- Tout ce qui dépend de SMPL/AMASS/HumanML3D/Human3.6M : WHAM, GVHMR, TRAM, 4DHumans, MotionBERT,
  **RTMW3D/RTMPose3D** (H3WB ← Human3.6M), tous les modèles texte → mouvement (MDM, MoMask,
  MotionGPT…).
- DeepLabCut SuperAnimal (chevaux) : recherche seule sans accord EPFL ; Horse-10 CC-BY-NC.
- Sapiens (CC-BY-NC), OpenPose (NC), YOLO-Pose (AGPL embarqué), Bandai Namco (CC BY-NC),
  LAFAN1 (NC-ND [?]).
- Mixamo, Rokoko, MoCap Online : fichiers non redistribuables dans un dépôt public.
- Extraits de films sous droits : reproduction soumise à autorisation (citation audiovisuelle
  étroite en droit français) ; s'inspirer d'un mouvement sans copier la séquence est moins
  risqué mais à valider pour un usage commercial. Sources propres : vos tournages, Muybridge et
  films du domaine public, reconstitutions/HEMA sous CC-BY vérifiées vidéo par vidéo.

Aucun modèle texte→mouvement gratuit à licence commerciale propre trouvé.
