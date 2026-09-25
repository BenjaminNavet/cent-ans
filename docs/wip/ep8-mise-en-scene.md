# Lot EP8 — Mise en scène des batailles

Branche `worktree-agent-a8a54631be0d814c5`. Plan du chantier : `docs/wip/epic.md`. ADR : 0055
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
Toutes les étapes faites (squelette, cœur, lumière, nuages, poussière, fumées, oiseaux, plan
cinématique, captures, mesures, ADR 0055). `git merge main` (034351af, sans Rust) puis revérifié.

## Vérifications (après fusion de main)
cargo fmt, clippy `-D warnings`, `cargo test --workspace` (704 OK), `core/build.sh`, pytest
(549 OK), import Godot, `ep8_staging_test`, `ep2_horizon_test`, smoke entier (28 « smoke OK »,
code 0).

## Options
`--hour=<dawn|morning|midday|afternoon|dusk|night|h>` ; A/B : `--no-daytime`,
`--no-cloud-shadows`, `--no-staging-dust`, `--no-smoke`, `--no-birds`, `--no-cinematic`,
`--no-ep8` ; captures : `--birds-shot`, `--dust-shot`, `--cinematic-shot` (sans `--no-hud` pour
garder les bandes noires), `--cinematic`. Menu « Batailles de démonstration » : choix de l'heure.
Réglages : « Plan cinématique au premier choc », « Ralenti du plan cinématique ».

## Mesures (M4 Pro très chargé : charge 26-57, ~20 godot d'autres agents)
`tools/bench_ep1.sh <étiquette> --units=63 --bench-at=90 --hour=afternoon --weather=clear`
(15 008 soldats, palier epic 2400 × 1600, Haut) :

| Config | i/s moy. | médiane ms | p95 ms |
|---|---|---|---|
| A `--no-ep8` (avant) | 21,6 · 26,8 | 45,8 · 36,1 | 60,6 · 44,9 |
| B EP8 complet | 26,8 · 26,3 | 36,1 · 36,9 | 44,4 · 45,5 |
| `--no-daytime` / `--no-cloud-shadows` / `--no-staging-dust` / `--no-smoke` / `--no-birds` | 26,5 / 26,1 / 26,6 / 26,5 / 26,7 | 36,1-36,7 | 44,4-47,0 |
| crépuscule, EP8 | 26,4 | 36,4 | 46,3 |

Lecture : aucun écart mesurable entre A et B (bruit ±1 i/s) ; la machine plafonne tout à ~27 i/s
(médiane figée à 36 ms), donc la cible de 40 i/s n'est pas vérifiable ici : EP1 mesurait ~57 i/s
sur la même scène, machine moins chargée. À remesurer machine calme.

## Captures
`docs/img/ep8/` : `aube.png`, `crepuscule.png`, `charge_poussiere.png`, `envol_oiseaux.png`,
`plan_cinematique.png`.

## Points ouverts
- Mesure à refaire sur machine calme (cible 40 i/s au palier épique).
- EP6 : appeler `BattleScene.add_smoke_source(position, intensity)` pour ses camps et mettre
  `staging.auto_campfires = false` (sinon feux par défaut en plus).
- EP7 : fixer l'heure historique (`BattleSim.set_start_hour`, ex. Crécy en fin d'après-midi).
- Pas de lever/coucher selon la saison, pas d'étoiles ; villages du champ jamais en feu (S2 ne
  vit que dans les sièges) ; oiseaux petits à l'écran (taille réelle).
- `epic.md` (tableau des lots) à mettre à jour par l'orchestrateur à la fusion.
