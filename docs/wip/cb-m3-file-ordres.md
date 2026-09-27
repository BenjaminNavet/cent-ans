# CB-M3 — Ordres en file (état)

Branche : `feat/cb-m3-queue` (depuis `main` 88ec35be, qui contient CB-M2). Plan :
`docs/superpowers/plans/2026-09-27-controles-bataille-tw.md` (section CB-M3, écart 6).
Cible cargo privée : `CARGO_TARGET_DIR=<worktree>/core/target-cbm3`.

## État : terminé, en attente de relecture visuelle et de fusion (session principale)

- [x] Données : `data/rules/battle_queue.json` (`max_queued_orders` = 8) + schéma
      `battle_queue_rules.schema.json` + `tools/tests/test_battle_queue_schema.py`. Fichier à part
      plutôt que `battle_hover.json` : CB-M4 modifie ce dernier en parallèle.
- [x] Cœur : `Command::Move`/`Attack` + `queue` (`serde(default)`, omis du JSON quand faux :
      un rejeu antérieur se relit et un ordre simple s'écrit comme avant) ; `Unit.order_queue`
      (`VecDeque<QueuedOrder>`, omis du JSON si vide) ; `QueuedOrder`, `QueueRules`
      (`src/queue.rs`) ; `sim/queue.rs` (`start_move`, `start_attack` extraits de
      `apply_command` à l'identique, `check_queue_room`, `queue_anchor`, `next_queued`,
      `Unit::busy`) ; `CommandError::QueueFull` (« file d'ordres pleine : … ») ;
      `group_destinations_from` (points de départ = fin de file ; mêmes opérations flottantes
      qu'avant pour un ordre simple).
- [x] Dépilage (`resolve_movement`) : destination atteinte ; cible disparue ; cible en fuite
      **seulement si une file attend** (sans file, la poursuite reste celle d'avant) ; ordre
      coupé par une mêlée (la mêlée efface la destination) : l'ordre suivant part une fois le
      contact rompu. Un ordre d'attaque en file dont la cible a disparu ou fuit est sauté.
- [x] Vidage : ordre sans `queue`, halte, retraite, ordre de muraille, déroute, décision de fin
      (déroute, refus, accalmie), pavois, ralliement, placement de scénario.
- [x] `state_digest` : file hachée seulement si non vide (empreintes des rejeux antérieurs
      inchangées).
- [x] Aperçu : `preview_path_from(unit, from, x, z)` et `preview_group_queued` (cœur) ; pont
      `battle_sim_queue.rs` : `preview_path_from(id, fx, fz, x, z)`, `preview_paths_queued`,
      `get_units()[i].queue = [{x, z, facing?, target?}]` ; RuleValues `battle_queue_max`.
- [x] Godot : Maj + clic droit (et Maj + glisser-droit actuel, sans largeur) envoient
      `queue: true` ; ordres simples inchangés (pas de clé `queue`). `BattlePathPreview` : trajets
      segment par segment (`preview_path_from`, mis en cache), flèche rouge pour une attaque en
      file, points numérotés (Label3D, 1 = ordre en cours), fantôme au dernier point ; aperçu en
      direct avec Maj depuis la fin de file (`preview_paths_queued`), masquage de la file pendant
      l'aperçu comme avant. File pleine : rien d'envoyé + message, curseur `forbidden` et
      infobulle `BattleQueueTip` tant que Maj est tenue.
- [x] Tests : `sim-battle/tests/cb_queue.rs` (12), `game/tests/cb_m3_queue_test.gd`,
      capture `game/tests/cbm3_queue_shot.gd` (`--probe` texte : 3 ordres en file, numéros 1-4
      à l'écran, OK).

## Capture à produire (session principale)

`godot --path game --resolution 1600x900 --script res://tests/cbm3_queue_shot.gd -- --out=docs/img/cb/cbm3-queue.png`
(non exécutée ici, non relue).

## Prochaine étape

Relecture de la capture, fusion. Suites possibles : Maj + glisser-droit avec largeur (CB1),
file visible aussi pendant l'aperçu en direct avec Maj, addendum ADR 0095 (fichier de règle
`battle_queue.json`, dépilage sur cible en fuite seulement avec file).
