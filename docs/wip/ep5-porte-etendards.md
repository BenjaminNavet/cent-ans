# Lot EP5 — Porte-étendards (chantier « batailles épiques »)

Branche `worktree-agent-ae217ad35aaf7a266`. ADR : `docs/decisions/0034-porte-etendards.md`.
Plan d'ensemble : `docs/wip/epic.md`.

## État
| Partie | État |
|---|---|
| 0. Squelette, contrat | en cours |
| 1. Figurines Blender (porte-étendard à pied / à cheval, tambour, busine) + clips | à faire (sous-agent) |
| 2. Données (`data/rules/battle_standards.json`, `data/fx/battle_standards.json`) + schémas | à faire |
| 3. Règles cœur (chute, relève, prise, trophées) + tests | à faire |
| 4. Rendu Godot (figurines dédiées, drapeaux instanciés jusqu'à 600 m, tombé/pris) | à faire |
| 5. Musiciens (sons) | à faire |
| 6. Écran de fin + chronique | à faire |
| 7. Captures `docs/img/ep5/`, ADR | à faire |

## Contrat figurines (Blender → Godot)
Figurines du manifeste `game/assets/models/battle_skinned/manifest.json` (nom `kind_variant`) :
- `standard_0` : porte-étendard à pied (rig `human`), `style` = `standard`.
- `standard_1` : porte-étendard à cheval (rig `cavalry`), `style` = `standard_mounted`.
- `musician_0` : tambourin (rig `human`), `style` = `drum`.
- `musician_1` : busine / trompette (rig `human`), `style` = `horn`.
Pas d'arme ni d'écu ; tabard de livrée (code livrée + armoiries), hampe de 3,8 m tenue des deux
mains et portée par l'os virtuel `Prop` (pour les porte-étendards).
Entrée de manifeste en plus pour `standard_*` : `pole_top` = [x, y, z] (pointe de la hampe en
espace de repos Godot) et `pole_axis` = [x, y, z] (vecteur unitaire le long de la hampe, vers le
haut, en repos). Godot place l'étoffe au sommet par la matrice de `Prop` lue dans la texture d'os.
Clips `human` : `std_idle`, `std_walk`, `std_run`, `std_wave` (mêlée : brandir), `std_death`
(chute avec la hampe), `drum_idle`, `drum_march`, `drum_beat`, `horn_idle`, `horn_walk`,
`horn_blow`. Clips `cavalry` : `c_std_idle`, `c_std_walk`, `c_std_gallop`, `c_std_wave`,
`c_std_death`.

## Prochaine étape
Squelette commité ; lancer le sous-agent Blender ; règles cœur.
