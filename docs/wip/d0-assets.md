# D0 — acquisition d'assets libres (top 15 de l'audit A4)

Source : `docs/audit/a4-assets-libres.md`. Assets rangés sous `game/assets/third_party/<catégorie>/<nom>/`,
chacun avec un `SOURCE.md` (source, URL, licence, auteur, modifications). Crédits dans `CREDITS.md`
(section « Assets tiers »). **Rien n'est branché dans le jeu** (lots suivants). Pas de git LFS
(le dépôt ne l'utilise pas). Poids total : 82 Mo, aucun fichier > 20 Mo (le plus gros :
`pine_tree_01.glb`, 12,7 Mo).

## État : terminé

- [x] Polices EB Garamond, IM Fell English (#3, #4)
- [x] Kenney Fantasy UI Borders (#5), Castle Kit (#7)
- [x] Poly Haven HDRI Belfast Open Field (#8), Autumn Field Pure Sky (#9)
- [x] Poly Haven Fir Tree 01 + Pine Tree 01 (#10), Grass Medium 01/02 (#11), convertis en GLB
- [x] Parchment GUI (#12)
- [x] Quaternius : 4 personnages riggés (#1, sélection médiévale) + Horse, White Horse, Donkey (#2)
- [~] #6 Medieval Village MegaKit : **remplacé** par le Medieval Village Pack (39 modèles, Poly Pizza)
- [ ] #13, #14 SFX Freesound : **sautés** (original téléchargeable seulement connecté ; Chrome indisponible)
- [x] #15 Musiques incompetech (Lord of the Land, Village Consort)
- [x] Crédits `CREDITS.md`
- [x] Import Godot : `godot --headless --path game --import` sans erreur sur ces fichiers (seules
  erreurs : GDExtension non compilée dans le worktree, sans rapport) ; un script de contrôle a
  chargé les 443 ressources importées sans échec.

## Détail par asset

| Chemin (`game/assets/third_party/…`) | Licence | Contenu | Animations / squelette | Triangles | Remarques d'intégration |
|---|---|---|---|---|---|
| `characters/quaternius_modular_men/` | CC0 | `king.glb`, `adventurer.glb`, `hooded_adventurer.glb`, `farmer.glb` | 24 anims communes (Idle, Idle_Sword, Walk, Run, Run_Back/Left/Right, Sword_Slash, Punch_L/R, Kick_L/R, Roll, HitRecieve, HitRecieve_2, Death, Interact, Wave, Idle_Neutral + 5 anims d'arme à feu inutiles) ; armature `CharacterArmature`, 62 os identiques (Root, Body, Hips, Abdomen, Torso, Chest, Neck, Head, Shoulder/UpperArm/LowerArm/Wrist .L/.R, doigts complets, UpperLeg/LowerLeg/Foot .L/.R, PT.L/.R) | 11 100 / 10 198 / 7 276 / 5 476 | Squelette partagé : animations interchangeables, `AnimationLibrary` commune possible. Le pack « Ultimate Modular Men » est surtout moderne (SWAT, costume, astronaute) ; seuls ces 4 passent en médiéval. King = chevalier/seigneur en armure. Farmer porte salopette + chapeau (retexture requise). Pas d'animation de tir à l'arc ni de lance : `Sword_Slash` / `Punch` à réutiliser. Échelle : ~2,9 unités de haut (à ramener à 1,75 m). 7-11 k tris : trop lourd pour des centaines de soldats sans LOD/imposteurs. |
| `animals/quaternius_animated_animals/` | CC0 | `horse.glb`, `horse_white.glb`, `donkey.glb` | 13 anims (Walk, Gallop, Gallop_Jump, Idle, Idle_2, Idle_Headlow, Eating, Attack_Kick, Attack_Headbutt, Idle_HitReact_Left/Right, Jump_toIdle, Death), présentes en double (`AnimalArmature|X` et `X`) ; armature `AnimalArmature`, 50 os (dont IK jambes, queue 7 os) | 2 182 / 2 182 / 2 000 | Pas d'os de selle : le cavalier devra être attaché à `Torso`/`Back` via `BoneAttachment3D`. Chevaux ~5,8 unités de long au repos (à mettre à l'échelle). Filtrer les anims dupliquées à l'import. |
| `buildings/quaternius_medieval_village/` | CC0 | 39 GLB : maisons, caserne, auberge, forge, moulin, scierie, écurie, clocher, charrette, tonneau, caisses, sacs, foin, puits, étals, clôture, feu | — | 84 à 10 120 (voir `SOURCE.md`) | Couleurs unies sans texture, style plus « cartoon » que la DA semi-réaliste ; utile pour props et placeholders de colonies. |
| `buildings/kenney_castle_kit/` | CC0 | 77 GLB (murs, tours, portes, pont, drapeaux, engins de siège) + `Textures/colormap.png` | — | faible (< 2 k par pièce) | Palette atlas unique : recolorable via la texture. Référence le fichier `Textures/colormap.png` relatif (garder la structure). |
| `vegetation/fir_tree_01/` | CC0 | 3 sapins (a, b, c) en nœuds racines distincts | — | 30 000 chacun | LOD2 Poly Haven décimé ; ~14-19 m de haut ; aiguilles en alpha MASK double face. Silhouette clairsemée après décimation ; prévoir imposteurs/billboards au loin. |
| `vegetation/pine_tree_01/` | CC0 | 3 pins (a, b, c) | — | 30 000 chacun | Idem ; 15-20 m. |
| `vegetation/grass_medium_01/` | CC0 | 10 touffes (large/mid/small/tall) | — | 121 à 1 032 | Candidates MultiMesh (prendre small/tall pour la densité). |
| `vegetation/grass_medium_02/` | CC0 | 5 touffes (a-e) | — | 714 à 2 489 | Plus lourdes ; plutôt pour gros plans. |
| `skies/belfast_open_field/` | CC0 | HDRI 2k `.hdr` (6,5 Mo) | — | — | Ciel couvert ; `PanoramaSkyMaterial` pour menu/cinématique. |
| `skies/autumn_field_puresky/` | CC0 | HDRI 2k `.hdr` (4,4 Mo) | — | — | Ciel dégagé sans sol : skybox neutre. |
| `fonts/eb_garamond/` | OFL 1.1 | TTF variables romain + italique (wght 400-800), `OFL.txt` | — | — | Corps de texte parchemin, remplace Georgia/Palatino ; renommés sans crochets. |
| `fonts/im_fell_english/` | OFL 1.1 | TTF romain + italique, `OFL.txt` | — | — | Titres ; nom réservé « IM FELL » (ne pas renommer une version modifiée ainsi). |
| `ui/kenney_fantasy_ui_borders/` | CC0 | 140 PNG (Default, Double : bordures, panneaux, séparateurs) | — | — | Blancs/gris à teinter sépia (`modulate` ou shader) ; `StyleBoxTexture` 9-patch. |
| `ui/parchment_gui/` | CC0 | 4 planches PNG (panneaux, boutons, étiquettes, emplacements) | — | — | Très basse résolution (≤ 176 px) : filtre `nearest` ou usage en petit. |
| `music/kevin_macleod/` | CC BY 4.0 | `lord_of_the_land.mp3` (3:06), `village_consort.mp3` (3:35) | — | — | MP3 256 kbit/s (pas d'encodeur Vorbis local pour OGG). **Attribution obligatoire** à afficher dans l'écran de crédits (déjà dans `CREDITS.md`). |

## Échecs et écarts

- **Freesound** (Church Bell, Swords Clash, CC0 vérifiés sur la page) : le téléchargement de
  l'original redirige vers la connexion ; l'extension Chrome n'était pas connectée. Consigne :
  sauter. Option pour un lot suivant : télécharger connecté, ou utiliser la preview HQ MP3
  (`cdn.freesound.org/previews/425/425172_8323418-hq.mp3`, `…/364/364531_3908740-hq.mp3`),
  licence CC0 identique mais qualité réduite.
- **Medieval Village MegaKit** : uniquement sur itch.io (flux JS) ; remplacé par le Medieval
  Village Pack. Les dossiers Google Drive publics de Quaternius (Knight, RPG Characters,
  Animals) renvoient « quota dépassé ». Le **Knight Pack** et le **RPG Character Pack**
  (Warrior, Ranger, Cleric…), plus médiévaux, restent à récupérer (Drive plus tard ou navigateur).
- MCP Blender non joignable (addon non lancé) : conversions faites en `blender -b` (scripts
  `tools/blender_scripts/polyhaven_vegetation.{sh,py}` et `glb_alpha_mask.py`, reproductibles).
- Le crash machine a vidé le scratchpad : musique et végétation re-téléchargées.
