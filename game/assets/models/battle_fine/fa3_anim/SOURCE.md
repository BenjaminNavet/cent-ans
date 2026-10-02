# Animations libres reciblées (lot FA3)

Clips cuits (`human.bones.bin`, `CAB1`, os du rig fin `human`) par
`tools/blender_scripts/fa3_anim_retarget.py` d'après la table
`data/fx/fa3_anim_sources.json`, lus seulement avec `--fa-anim` après `--` (par-dessus les
clips de mêlée par défaut NT14). Source, licence et mesures de chaque clip : `manifest.json`
(`clip_sources` : glissement des pieds, écart de boucle, bouclier devant le visage, et les
mêmes mesures sur le clip remplacé).

## Sources (CC0 1.0, domaine public ; aucune attribution requise, crédit de courtoisie)

- **Mesh2Motion**, animations humaines (base, add-on, capture de mouvement) — Scott Petrovic
  et contributeurs. https://github.com/Mesh2Motion/mesh2motion-app (fichiers
  `human-{base,addon,mocap}-animations.glb` de l'application). README du dépôt : « The art
  assets (3d models, rigs, animations) are all licensed under CC0 ».
  Clips : `guard` (Idle_Shield), `parry` (Shield_OneShot), `hit` (Hit_Chest), `hit_b`
  (Hit_Head), `death` (Death_D), `death_back` (Death_B), `bow_shoot` (Bow Pull Back / Hold /
  Release), `victory` (Cheer_One_arm), `victory_b` (Cheering_Two_Hands).
- **KayKit Character Animations 1.1**, Rig_Medium — Kay Lousberg, www.kaylousberg.com
  (https://kaylousberg.com/game-assets/character-animations). Licence dans le paquet :
  `KayKit-CC0-License.txt` (Creative Commons Zero).
  Clips : `slash` (Melee_1H_Attack_Slice_Diagonal), `overhead` (Melee_1H_Attack_Chop),
  `pike_level` (Melee_2H_Idle), `pike_thrust` (Melee_2H_Attack_Stab).

Seuls des dérivés cuits (matrices d'os du rig du jeu) sont versionnés ; les fichiers GLB
d'origine restent hors dépôt (`~/dev/cent-ans-raw/fa/anim/`).
