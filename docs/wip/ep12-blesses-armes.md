# Lot EP12 — Blessés qui rampent, fuyards qui jettent leurs armes

Branche `worktree-agent-ab10bccc115923724`. Rendu seulement (game/, tools/blender_scripts, data/fx) ;
aucun changement du cœur. ADR 0070 (renumérotée 0073 → 0070 à la fusion : CT1 a repris 0073 ; 0071 laissée à EP11).

## Fait
1. Kit Blender (`battle_skinned_poses.py`) : clips `crawl` (7 s), `wounded_sit` (6 s),
   `wounded_kneel` (6 s), non bouclés et immobiles à la fin ; `flee` / `flee_m` (course de fuite
   mains vides, regard en arrière, sur `Run`). `battle_skinned.py` : drapeau de face
   `HELD_MASK = 64` sur les armes tenues et boucliers (`HELD_ITEMS`, pas les targes dans le dos) ;
   `--preview … --wide` (caméra qui suit le corps, sol) et aperçu fidèle (pose figée sans l'action
   au rendu). Défauts ruff de `battle_skinned_poses.py` corrigés (D205/D209).
2. Régénération : `blender --background --python tools/blender_scripts/battle_skinned.py -- --only
   infantry_0,…,infantry_8,archer_0,…,archer_5` → `human.bones.bin` (+5 clips à la fin, anciens
   clips identiques octet pour octet), 45 maillages à pied, manifeste ; `cavalry.bones.bin` inchangé.
3. Shader `battle_soldier_skinned.gdshader` : bits 0-5 = variantes ; `drop_arms` et le drapeau 8
   (`CODE_UNARMED`) du code de INSTANCE_CUSTOM.w masquent les faces `HELD_MASK`.
4. `BattleSkinned` : jeux `routing` des styles à pied = `flee`/`flee_m` (repli `run`),
   `wounded_config`, `style_of`, `WOUNDED_FOOT`, `CODE_UNARMED`.
5. `BattleSoldiers` : part des pertes tirée blessée (hachage id × effectif restant de la
   simulation × rang : déterministe, rejeu EP13 compris), couche de cellules `…/wounded`,
   plafonds `max_total` / `max_animated` ; morts d'un régiment débandé désarmés ; déroute →
   `drop_arms` + objets au sol (`BattleDroppedArms`, nouveau). `--no-ep12` (A/B).
6. `data/fx/battle_gore.json` : sections `wounded`, `dropped_arms`, cadence `flee` ; schéma à jour.
7. `battle_scene.gd` : `--ep12-shot=<wounded|rout>`, champs du banc `wounded`, `disarmed_units`,
   `dropped_arms`. Test/capture hors simulation `tests/ep12_shot.gd` (sans `--out` : contrôle
   `EP12_CHECK`).
8. Captures `docs/img/ep12/` : `wounded_close.png`, `rout_close.png` (hors simulation),
   `wounded_battle.png`, `rout_battle.png` (bataille).

## Mesures (Mac M4 Pro, charge 20-35, `tools/bench_ep1.sh … --units=63`, 15 053 soldats)
| Instant | Avant EP12 | Après EP12 | `--no-ep12` |
|---|---|---|---|
| `--bench-at=90` | 30,5 i/s, p95 36,2 ms, 984 appels | 30,8 i/s, p95 34,7 ms, 985 appels | — |
| `--bench-at=300` | 32,6 i/s, p95 33,3 ms, 997 appels | 32,2 · 32,8 i/s, p95 34,7 · 33,7 ms, 1 009-1 012 appels | 32,4 i/s, 997 appels |
À 300 s : 196 blessés, 9 régiments désarmés, 806 armes au sol. Écart dans le bruit.

## Coordination EP13
Le saut en arrière du rejeu libère `soldiers` (blessés et armes au sol en sont les enfants) puis
le reconstruit : purge propre. Le tirage des blessés dépend de l'effectif restant de la
simulation, pas d'un compteur du rendu. `ep13_replay_test` OK après fusion de main.

## Points ouverts
- Armes au sol à plat (pas d'inclinaison sur la pente) ; un régiment rallié « ramasse » ses armes.
- Cavaliers non concernés (ni blessés au sol, ni lances jetées).
- Le choix de la figurine qui tombe reste tiré par `_rng` (comme avant EP12).

## Prochaine étape
Fusion par l'orchestrateur (ADR 0070, ligne EP12 dans `docs/wip/epic.md`).
