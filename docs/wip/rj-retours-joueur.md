# RJ — retours joueur du 03/10

Retours : boutons d'ordre sans état on/off ; une seule formation cyclée sans noms historiques ; démarche saccadée ; mise en formation instantanée ; territoire illisible en campagne ; possession vs occupation confuse.

Décisions joueur (03/10) : règle de conquête **gardée** (la cité donne le contrôle, la possession seulement par traité/cession, rendue à la paix) mais **expliquée** ; lisibilité = **remplissage par position diplomatique** en vue 3D + **bannières sur les villes** (possesseur, occupant si différent).

Diagnostic :
- Boutons : `battle_hud.gd` COMMANDS sans état ; les modes ont déjà `mode_button_state`.
- Formations : `_next_formation` codé en dur (`battle_input.gd:498`) ; `enum Formation` (sim-battle `unit.rs:16`) ; ADR 0095 « pas de nouveau type de formation » → révisé par ADR 0174.
- Instantané : `BattleSim::apply` (`sim.rs:1271`) affecte `formation` directement ; places recalculées chaque image → téléportation.
- Saccade : DT 0,1 s, `PoseCache` non interpolé (`battle_sim.rs:275`), positions copiées brutes (`battle_soldiers.gd:625`).
- Carte : remplissage 3D `faction_alpha_near` 0,04, couleur héraldique du propriétaire ; aucune infobulle occupé/possédé.

ADR réservés : 0174 (formations historiques et reformation), 0175 (lisibilité du territoire).

## Lots

| Lot | Contenu | Worktree | État |
|---|---|---|---|
| a | Formations historiques (données + core), menu avec infobulles, reformation progressive, état on/off des boutons d'ordre | `../gp-rj-a` | lancé |
| b | Interpolation des poses entre pas de simulation (démarche) | `../gp-rj-b` | lancé |
| c | Explication conquête (infobulle province, panneaux, codex) + bannières possesseur/occupant sur les villes | `../gp-rj-c` | fusionné (a40fe9a90) ; en_stance_cues_test rouge déjà sur main avant RJ-c (anneau rouge des armées ennemies) |
| d | Remplissage par position diplomatique en vue 3D | `../gp-rj-d` | fusionné (321809b0f) |

## Prochaine étape
Relire et fusionner chaque lot au retour (worktree `../gp-rj-merge`, puis `--ff-only`).
