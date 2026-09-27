# CB4 — Capacités actives (état)

Branche : `feat/cb4-abilities` (depuis `main` 07eff33b, après « docs: CB wave 4 merged, CB4 next »).
Plan : `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md` (§ CB4) ; relecture
historique : `docs/research/cb4-capacites.md`. Cible cargo privée :
`CARGO_TARGET_DIR=<worktree>/core/target-cb4`.

## État : terminé, en attente de capture et de fusion

- [x] Données : `data/battle_abilities/*.json` (5 capacités : `ability_aimed_shot`,
      `ability_pavise`, `ability_banner_rally`, `ability_close_ranks`, `ability_planted_pikes`),
      schéma `battle_ability.schema.json`, `tools/tests/test_battle_abilities_schema.py` ;
      `order_pavise.json` retiré de `battle_orders` (fiche Codex `cdx_jeu_pavois` → entité
      `ability_pavise`, préfixe `ability` admis par `codex.schema.json` et `codex.py`).
- [x] data-model : `BattleAbility` (+ `GameData.battle_abilities`), chargée comme les ordres ;
      `BattleSetup.abilities` rempli par `battle_request.rs` (campagne) et
      `historical_battles.rs` (pont) ; `HistoricalMap::battle_setup` la laisse vide (l'appelant
      l'ajoute : `setup.abilities = data.battle_abilities…`).
- [x] Cœur : `sim-battle/src/abilities.rs` (éligibilité, recharge comptée depuis la fin,
      durée, délai de pose, conditions `stationary`/`not_engaged`/`ammo_gt_0` +
      `start_conditions`, arrêt avec raison, levée par un second emploi, une capacité à la fois) ;
      `Command::UseAbility { units, ability }` additive ; `Unit.ability_state` omis du JSON
      quand vide ; effets dans `sim.rs` : `effective_range`, `speed`, `fire` (précision,
      cavaliers, recharge, couverture `missile_cover`), `melee_damage`
      (`ability_melee_factor`), `charge_impact` (piques plantées, rangs serrés).
- [x] IA `ai_abilities.rs` (sous-module de `ai`, une règle par type, seuils dans le bloc `ai`
      de chaque fichier) ; `scenario_filter` (EP7) filtre `UseAbility` comme les ordres du chef.
- [x] Tests `sim-battle/tests/cb4_abilities.rs` (21 + sonde ignorée `probe_margins`) ;
      `tests/orders.rs` garde l'ordre pavois (JSON d'avant CB4) pour la règle des vieux rejeux.
- [x] Pont : `battle_sim_abilities.rs` (`get_units.abilities`, `get_ability_catalog`,
      `use_ability(units, ability)`, RuleValues `<id>_cooldown|_duration|_setup_time`,
      `<id>_<effet>_percent`, `<id>_morale_bonus|_morale_recovery`).
- [x] Godot : `battle_ability_icons.gd` (glyphes en code, cadran, infobulle RuleValues) ;
      `unit_card.gd` : rangée de 1 à 3 boutons sous la vignette (carte 94 → 112 px,
      `BattleHud.BAND_HEIGHT` 128 → 140) ; Alt/Option+1…4 (`BattleHotkeys.ability_slot`,
      `BattleInput.ability_command`, ligne `pending` levée) ; aide F1 par la table ; barre des
      ordres du chef à quatre raccourcis (Z X V B ; « Pas de quartier » passe de N à B).
- [x] Tests Godot `cb4_abilities_test.gd` ; capture `cb4_abilities_shot.gd` (`--probe` OK).

## Marges (release, graines de `ep7_historical` / `eq7_cavalry`)

Mesurées par `cargo test --release -p sim-battle --test cb4_abilities -- --ignored --nocapture
probe_margins` (et `cb2_modes … reference_margins`, qui passe maintenant le catalogue).

| Bataille | Bande | Avant (main) | 1re règle IA | Finale |
|---|---|---|---|---|
| Crécy | 14-19/20 | 16 | 18 | 18 |
| Azincourt | 14-19/20 | 18 | **20** | 18 |
| Poitiers | 11-18/20 | 14 | 16 | 16 |
| EQ7 | ≥ 12/16 | 15 | 13 | 15 |

Par capacité seule (règle finale) : pavois 16/18/14/15 (identique à main), tir tendu 18/19/14/15,
bannière 16/18/14/15, rangs serrés 16/18/16/14, piques 16/18/14/15.

## Notes d'équilibrage et choix

- L'ordre `order_pavise` ne se déclenchait jamais dans les batailles de référence : le filtre de
  scénario EP7 écarte les ordres des régiments tenus ou lancés à l'assaut. `UseAbility` passe par
  le même filtre : à Crécy, les Génois de l'IA ne dressent pas plus leurs pavois qu'avant
  (résultat identique graine par graine, pavois seul = chiffres de main).
- **Tir tendu** : la règle d'IA « cavalier à moins de 110 m » donnait Azincourt 20/20 ; limitée au
  tir de flanc, Crécy tombait à 4/20 (la portée réduite 30 s prive les archers de la masse qui
  arrive). Règle retenue : **seulement pour un camp à l'attaque** (`when_attacking`) : une ligne
  d'archers sur la défensive garde ses volées longues (l'IA ne sait pas peser l'échange).
- **Pavois** : de flanc ou avant la pose, la couverture passive (× 0,6 à l'arrêt) reste ; la
  capacité (× 0,35) ne vaut que de face, une fois plantée (4 s).
- **Rangs serrés** : pas de résistance à la poussée (EP11) ajoutée ; « pas de course » = la course
  est coupée tant qu'elle dure. Durée 60 s, recharge 30 s.
- **Tir tendu des archers montés** : éligibles seulement à pied (`mounted: false`), or aucun ordre
  ne les démonte aujourd'hui (le pied à terre vise la cavalerie) : sans effet pour l'instant.
- **Empreinte de rejeu** : l'état des capacités n'est pas haché (comme les modes CB2) ; ses effets
  le sont. Un rejeu avec capacités se rejoue exactement (test).
- Sans catalogue dans le `BattleSetup` (vieux rejeux, installations faites à la main), aucune
  capacité : les tests existants qui construisent leur bataille à la main sont inchangés.

## Capture (session principale)

`godot --path game --resolution 1600x900 --script res://tests/cb4_abilities_shot.gd -- --out=docs/img/cb/cb4-capacites.png`
(non exécutée ici, non relue ; `--probe` : actives 2, en recharge 2, grisée 1 « au contact de l'ennemi »).

## Suites

- Icônes DA5 des capacités (glyphes en code pour l'instant ; `data/ui/icons_ink.json` cible
  encore `order_pavise` pour l'icône du pavois).
- Addendum ADR 0095 (capacités, filtre de scénario, règle d'IA du tir tendu).
- Pieux : passif inchangé (datation hors CB, suites de l'historien).
