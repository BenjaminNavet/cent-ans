# Lot EP8 — Mise en scène des batailles

Branche `worktree-agent-a8a54631be0d814c5`. Plan du chantier : `docs/wip/epic.md`. ADR : 0052
(à écrire ; 0053 est pris ailleurs, vérifier avant de commiter).

## Conception
- **Heure du jour (cœur)** : `data/rules/battle_time_of_day.json` (schéma
  `battle_time_of_day_rules.schema.json`) ; `sim-battle/src/time_of_day.rs` (`TimeOfDayRules`,
  phases aube/matin/midi/après-midi/crépuscule/nuit, visibilité, 0,2 min de jour par seconde de
  bataille, tirage campagne pur par hachage tour/indice/province) ; `sim/time_of_day.rs`
  (`BattleSim::set_start_hour`, `hour`, `day_phase`, `visibility`, `range_factor` = météo ×
  lumière, annonce de la phase au journal). `effective_range`, tir sur muraille, guetteurs du tir
  indirect et IA lisent `range_factor()`. Défaut midi : batailles et graines existantes inchangées.
- **Pont** : `setup` lit la clé `hour` du dictionnaire ; `get_battle_setup` (campagne) l'ajoute ;
  `set_start_hour`, `set_start_phase(key)`, `get_time_of_day()`.
- **Rendu** (`data/fx/battle_staging.json`, schéma `fx_battle_staging.schema.json`) :
  `battle_staging.gd` orchestre lumière par heure (images clés), ombres de nuages (décal + texture
  de bruit défilant au vent), poussière enrichie (B4), fumées (`add_smoke_source`), oiseaux,
  caméra cinématique.

## État
| Étape | État |
|---|---|
| 0. Données, schémas, test pytest, cœur heure du jour + tests (`tests/ep8_time_of_day.rs`), pont | fait |
| 1. Lumière selon l'heure (Godot) + `--hour=` + menu démos | fait (capture aube OK) |
| 2. Ombres de nuages | fait (décal + bruit périodique CPU ; ViewportTexture refusée par Decal) |
| 3. Poussière enrichie (effectif, terrain, colonnes) | fait, à vérifier en capture |
| 4. Fumées (`add_smoke_source`, bombardes, incendies S2) | fait, à vérifier en capture |
| 5. Oiseaux, corbeaux | fait, test `game/tests/ep8_staging_test.gd` OK |
| 6. Caméra cinématique, ralenti, réglages | fait, à vérifier en capture |
| 7. Mesures A/B (`tools/bench_ep1.sh`), captures `docs/img/ep8/`, ADR | à faire |

## Prochaine étape
Captures (`--birds-shot`, `--cinematic-shot` à brancher dans `_stage_screenshot`), crépuscule,
mesures A/B (`tools/bench_ep1.sh`), ADR 0052, smoke complet.
