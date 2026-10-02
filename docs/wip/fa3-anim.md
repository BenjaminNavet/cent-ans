# FA3 — animations de combat CC0 reciblées sur les figurines fines

Branche `feat/fa-anim` (worktree `../gp-fa-anim`, issue de `feat/fa`). Chantier : `docs/wip/fa.md`.
Prédécesseurs : NT12 (`docs/wip/nt12-mocap.md`), NT13/NT14 (`docs/wip/nt13-video-mocap.md`).

## État : TERMINÉ (10-02), prêt à juger par la session principale
Les clips par défaut ne changent pas ; tout passe par `--fa-anim` après `--`.

## Fait
- Table `data/fx/fa3_anim_sources.json` (sources, squelettes, clips du jeu <- clips source,
  options), schéma `data/schemas/fx_fa3_anim_sources.schema.json`, test
  `tools/tests/test_fa3_anim_sources_schema.py` (4 tests).
- `tools/blender_scripts/fa3_anim_retarget.py` : import glTF natif, reciblage par rotations
  (formule NT12, fonctions de `nt12_mocap_trial` réutilisées : `Target`, `to_basis`,
  `blend_basis`, `quality`, et `bake_clip` / `write_bake` factorisées hors de `nt12.bake`, cuisson
  NT12 identique octet pour octet), hanches et chevilles placées d'après la source mise à
  l'échelle, IK à deux os des deux jambes sur toutes les images, boucles sans dérive, clé de
  bouclage répétée ôtée, options par clip (bras du bouclier, sol, arme d'hast, arc).
- Cuisson `game/assets/models/battle_fine/fa3_anim/` (13 clips, 472 images, 0,4 Mo) +
  `manifest.json` (source, licence, mesures FA3 et mesures du clip remplacé) + `SOURCE.md`.
- `BattleSkinned` : `--fa-anim`, `fa_anim_forced`, couche posée **par-dessus** la couche en
  place (défaut NT14 ou essai) : `mocap_textures` est une liste, `thrust` garde NT14.
- `game/tests/fa3_anim_test.gd` ; les tests NT12/NT13/NT14 forcent `fa_anim_forced = 0`.
- Planches (hors dépôt) : `~/dev/cent-ans-raw/fa/anim/boards/fa3_{melee,polearm,bow,cheer}.jpg`
  par `fa3_anim_retarget.py -- render DIR` puis `fa3_anim_board.py DIR BOARD_DIR`.

## Commandes
```
blender -b --factory-startup --python tools/blender_scripts/fa3_anim_retarget.py
blender -b --factory-startup --python tools/blender_scripts/fa3_anim_retarget.py -- render DIR [clips]
uv run --project tools python tools/blender_scripts/fa3_anim_board.py DIR ~/dev/cent-ans-raw/fa/anim/boards
CENT_ANS_FA3_TABLE=autre.json blender … -- render DIR   # essai de clips candidats (rendus)
```

## Constats
- **Pose de repos du rig fin = contrapposto** (hanches tournées de 27°, pied gauche reculé et
  ouvert de 59°, genoux fléchis). `nt12.Target` en tire ses axes : tout reciblage NT12 est
  donc tourné de 27° et penché de 5° (défaut jamais vu de NT12). FA3 prend les axes du monde
  (`target()`), remet hanches et buste d'équerre, aligne cuisses et tibias avec la charnière
  du genou, tourne les pieds vers l'avant, et place les joints en absolu (pas en écart au repos).
- Clips Mesh2Motion à coup unique : la dernière image répète la première (gardée : le clip
  revient à son départ) ; boucles : image répétée ôtée, aucun fondu nécessaire (écart 0).
- Certaines actions ne posent pas toutes les pistes : pose remise à zéro avant chaque image.
- `Sword_Regular_A/B/C` avancent de 0,8 à 1,1 m (os `root` animé) ; `Sword_Attack` tombe un
  genou à terre et change d'appui ; `Idle_Sword` est une garde très basse : écartés. **Écart au
  brief** : `slash` et `overhead` viennent donc de KayKit (coups sur place), pas de Mesh2Motion.
- KayKit a des jambes courtes : déplacements mis à l'échelle par la hauteur de tête
  (`scale_by: head`) ; bras du bouclier repris de la garde FA3 (`left_arm: clip_guard`).
- Arme d'hast : KayKit tient l'arme en travers ; le corps est tourné pour que la ligne des
  poings pointe devant (`prop.aim: hands`), la pique garde l'axe du clip keyframé
  (`prop.axis`), portée par le poing droit, main gauche posée dessus par IK.
- Arc : arc tenu vertical (`bow_upright`), corde tirée par le poing droit des images 12 à 37,
  décoche à l'image 37 (1,55 s, mode VOLLEY), buste incliné de 32° (élévation des flèches).

## Mesures (manifeste ; FA3 / clip remplacé)
| clip | source | images | glissement pieds cm | écart boucle cm | bouclier devant le visage |
|---|---|---|---|---|---|
| guard | M2M Idle_Shield | 60 / 25 (NT14) | 0,3 / 0,0 | 0,3 / 0,5 | 0 image (22 cm) / 0 (42 cm) |
| slash | KayKit Slice_Diagonal | 21 / 25 | 27,5 (= source, pas d'appui) / 14,6 | — | 0 / 0 |
| overhead | KayKit Chop | 18 / 25 (NT14) | 0,2 / 0,0 | — | 0 / 0 |
| parry | M2M Shield_OneShot | 24 / 38 (NT14) | 0,0 / 1,3 | — | 0 image (23 cm) / 0 |
| hit | M2M Hit_Chest | 14 / 14 | 0,6 / 0,0 | — | 0 / 0 |
| hit_b | M2M Hit_Head | 14 / 14 | 0,0 / 0,0 | — | 0 / 0 |
| death | M2M Death_D | 53 / 26 | 2,2 / 31,5 | — | 0 / 0 |
| death_back | M2M Death_B | 45 / 26 | 9,4 / 13,9 (et 20 cm sous le sol) | — | bras sur le visage au sol |
| pike_level | KayKit 2H_Idle | 26 / 41 | 0,0 / 0,0 | 0,0 / 0,0 | — |
| pike_thrust | KayKit 2H_Stab | 28 / 28 | 36,0 (= source) / 0,0 | — | — |
| bow_shoot | M2M Bow Pull/Hold/Release | 60 / 60 | 0,0 / 0,0 | — | — |
| victory | M2M Cheer_One_arm (mocap) | 52 / 41 | 0,9 / 0,0 | 1,9 / 0,0 | 0 / 0 |
| victory_b | M2M Cheering_Two_Hands (mocap) | 38 / 82 | 1,7 / 0,0 | 1,6 / 0,0 | 0 / 0 |

Le reciblage n'ajoute aucun glissement (`foot_slide_cm` = `foot_slide_source_cm` partout, cible
de cheville atteinte à 0,0 cm) : les valeurs non nulles sont des pas de la source près du sol.

## Défauts constatés (planches lues)
- `pike_level` / `pike_thrust` : genoux rentrés (jambes KayKit), buste de profil, poussée de
  21 cm seulement (39 cm en keyframé), le soldat pivote beaucoup pendant l'estoc.
- `slash`, `overhead` : gestes sobres mais stylisés (KayKit), fin de clip à 20 cm du départ
  (le fondu de cycle de 0,2 s la couvre) ; clips courts (18-21 images) tenus sur leur dernière
  pose jusqu'à la fin du cycle de 1,3 s.
- `hit` : bras droit de la pose neutre Mesh2Motion (épée le long du corps) ; à-coup mesuré plus
  fort que le keyframé (clip de 10 images étiré à 14).
- `bow_shoot` : la corde et la flèche (os virtuels) ne se voient pas dans Blender ; à juger en
  jeu. Retour à la première pose par fondu (11 images).
- `guard` : bouclier haut devant le buste, visage dégagé (centre du bouclier à 22 cm de la ligne
  de regard au plus près, seuil d'alerte 22 cm) ; garde plus fermée que NT14.
- Poignets : l'arme suit le poignet droit reciblé, aucune correction de prise.

## Tests
- pytest `test_fa3_anim_sources_schema.py` : 4 OK. `ruff check` et `ruff format` : propres.
- Godot : `fa3_anim_test` (défaut, `--fa-anim`, `--coarse-figures`), `nt7_anim_test`,
  `an1b_clips_test`, `nt12_mocap_test` (aussi `--mocap-trial`), `nt13_video_test`,
  `nt14_melee_test`, `smoke` : OK avec et sans `--fa-anim`.

## Prochaine étape (session principale)
Juger les planches puis en jeu (`godot --path game -- --fa-anim`), choisir clip par clip ce
qui devient défaut (même mécanique que `melee/` de NT14 : une table de choix et une cuisson).
Pistes : `thrust` (KayKit Melee_1H_Attack_Stab), `hit_c`, `death_knees` (Death_A), fantassins
du kit grossier non reciblés, crédits dans `CREDITS.md` au lot FA4.
