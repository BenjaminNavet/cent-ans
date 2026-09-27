# CB2 — Modes d'unité, icônes d'état, remappage des touches (état)

Branche : `feat/cb2-modes` (depuis `main` 220a1e9f). Plan :
`docs/superpowers/plans/2026-09-27-controles-bataille-tw.md` (section CB2). Fusion **en dernier** de
la vague 4 (après CB3, CB5, CB6). Cible cargo privée : `CARGO_TARGET_DIR=<worktree>/core/target-cb2`.

## État : terminé, vérifications finales en cours

- [x] Données : `data/rules/unit_modes.json` (course, escarmouche, seuils d'état, brèche, IA) +
      schéma `unit_modes_rules.schema.json` + `tools/tests/test_unit_modes_schema.py`.
- [x] Cœur :
  - `src/modes.rs` : règles (`UnitModeRules`), `UnitMode` (run, guard, skirmish, melee, breach),
    `UnitStatus` (charging, under_fire, engaged, wavering) ;
  - `src/sim/modes.rs` : `set_mode` (disponibilité par régiment, exclusions garde/escarmouche et
    escarmouche/mêlée), recul d'escarmouche (`resolve_skirmish`, avant les mouvements), garde
    (`guard_releases`), brèche (`breach_piece`, dégâts et recharge), `unit_status`,
    `available_modes` ; `Unit::shoots()` / `shoots_on_move()` ;
  - `src/ai_modes.rs` (sous-module de `ai`) : IA défensive ;
  - champs `Unit.{mode_run, guard, skirmish, melee_mode, breach}` omis du JSON quand faux ;
    `skirmish` vrai au départ pour la capacité `skirmish` ; `Command::SetMode` additif ;
    `CommandError::ModeUnavailable`.
- [x] Pont : `battle_sim_modes.rs` (`get_units` : `mode_run`, `guard`, `skirmish`, `melee_mode`,
      `breach`, `modes`, `charging`, `under_fire`, `engaged`, `wavering`) ; RuleValues
      `unit_modes_*` ; `issue_command` accepte `{type: "set_mode", units, mode, enabled}`.
- [x] Godot :
  - `battle_hotkeys.gd` : **table unique** des raccourcis (aiguillage, libellés, aide F1) ;
  - `battle_input.gd` : F tir à volonté, T formation, R course, G garde, K escarmouche, M mêlée,
    Ctrl+G verrou (via la table) ; `mode_command` (pur) ;
  - `battle_hud.gd` : aide F1 générée (`{keys}`), boutons de mode (grille 5 colonnes), état doré
    ou grisé selon la sélection (`mode_button_state`, pur) ;
  - `battle_mode_icons.gd` : glyphes des modes et des états, dessinés en code ;
  - `battle_unit_markers.gd` : pastilles déroute > hésite > sous le feu > charge > mêlée > mode >
    tir > épuisée, trois au plus ; `unit_card.gd` : glyphes des modes actifs, infobulle « Modes » ;
  - aide de campagne (`help_controller.gd`) et fiche Codex `cdx_jeu_tir` : touche F.
- [x] Tests : `sim-battle/tests/cb2_modes.rs` (18 + sonde des marges ignorée),
      `game/tests/cb2_modes_test.gd`, capture `game/tests/cb2_modes_shot.gd` (`--probe` OK).
- [x] Golden CB0 **inchangé** : la séquence du test presse T puis F (au lieu de F puis G), mêmes
      ordres.

## Marges des batailles de référence (release, graines du test)

Mesurées par `cargo test --release -p sim-battle --test cb2_modes -- --ignored --nocapture
reference_margins` (mêmes graines que `ep7_historical` et `eq7_cavalry`) ; « avant » = règle d'IA
des modes coupée (comportement de `main`, `ep7_historical` et `eq7_cavalry` verts sur `main`).

| Bataille | Bande | Avant | Après (1re règle IA) | Après (finale) |
|---|---|---|---|---|
| Crécy | 14-19/20 | 16 | 16 | 16 |
| Azincourt | 14-19/20 | 18 | 18 | 18 |
| Poitiers | 11-18/20 | 14 | **6** (puis 5) | 14 |
| EQ7 | ≥ 12/16 | 15 | 15 | 15 |

La garde gardée en plein combat par la ligne défensive de l'IA faisait tomber Poitiers à 6/20
(plus de poursuite des fuyards ni des régiments qui rompent) : l'IA ne met sa ligne en garde que
tant qu'elle attend (sans cible, hors mêlée) ; un régiment lancé sur l'ennemi quitte la garde.

## Choix / écarts

- **Empreinte de rejeu** : les drapeaux de mode ne sont pas hachés (seuls leurs effets le sont :
  positions, effectifs, états). Hacher les drapeaux faisait diverger dès 10 s l'échantillon EP13
  antérieur à CB (l'IA défensive met sa ligne en garde) alors que la bataille montrée est la même.
  Un rejeu avec des modes se rejoue exactement (test).
- **Garde** : lâche une cible en déroute, qui rompt le contact (`disengaging`) ou se retire, hors
  contact seulement (une poussée qui sépare les lignes un instant n'est pas une rupture).
- **Escarmouche** : tout tireur (hors engins) peut la prendre ; le tir en marche reste réservé à la
  capacité `skirmish` avec le mode actif. Le recul (bond de 50 m, déclenché à 60 m par une troupe
  de mêlée qui marche ou charge vers l'unité) n'a lieu qu'à l'arrêt ou en tir, sans ordre de
  déplacement en cours ni file ; la cible désignée est lâchée (le tir à volonté reprend).
- **Mêlée** : l'unité ignore ses armes de trait (`shoots()`), charge et engage.
- **Course** : multiplicateurs de vitesse et de fatigue à 1 dans les données (comme le double clic
  droit) ; la bascule change aussi l'allure du déplacement en cours.
- **Battre en brèche** : engins `wall_breaker` en siège seulement ; dégâts aux murs ×1,6
  (mangonneau ×1,3), recharge ×1,15 ; cible le mur ou la porte la plus proche à portée si aucun
  pan n'est désigné ; jamais de tir sur les hommes ; un ordre d'attaque met fin au mode. **Pas de
  règle d'IA** (le chercheur la recommandait chez l'assiégeant) : elle déplacerait l'équilibre des
  sièges (`sg4_balance`), hors du périmètre demandé → suite possible.
- **IA** : tireurs légers (armure ≤ 25, sans pieux ni pavois : archers montés, genétaires) en
  escarmouche en défense ; les arcs longs et les arbalètes gardent leur conduite (`plan_shooter`).
- **Brèche sans touche** (bouton seul) : B est une touche des ordres du chef.
- Glyphes en code ; icônes DA5 (`battle_mode_<mode>`, états) à générer par la session principale.

## Fusion (CB2 en dernier)

Les raccourcis des autres lots entrent dans `BattleHotkeys.BINDINGS` : déjà présents comme lignes
d'aide `Tab` (CB3, vue tactique), `Alt+Maj+1…6` (CB6), `Alt+1…4` (CB4, `pending: true` jusqu'à sa
fusion). En résolvant `HELP_TEXT` : garder le gabarit CB2 (`{keys}`) et déplacer les lignes de
touches ajoutées par CB3/CB5/CB6 dans la table. Dans `battle_input.gd`, l'aiguillage par la table
précède le `match` : une touche traitée par un autre lot (Tab, Alt+Maj+chiffres) ne doit pas avoir
de ligne `dispatch: true`.

## Capture (session principale)

`godot --path game --resolution 1600x900 --script res://tests/cb2_modes_shot.gd -- --out=docs/img/cb/cb2-modes.png`
(non exécutée ici, non relue).

## Prochaine étape

Vérifications finales (workspace, release, Godot), commit final.
