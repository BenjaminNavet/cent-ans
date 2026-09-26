# 0073 — Relecture du tour de l'IA : enregistrement des marches dans le cœur

Date : 2026-09-26. Lot CT1. Statut : accepté.

## Contexte

Depuis M3, `end_turn` résout d'un bloc le tour de chaque faction IA (marches reprises, ordres du
planificateur, batailles auto-résolues). Le joueur ne voyait que l'état final : les armées IA se
« téléportaient ». Total War montre au contraire les armées ennemies visibles marcher, la caméra
se portant sur celles qui menacent le joueur. Il faut le trajet réel de chaque armée, et savoir
lequel concerne le joueur, sans mettre de règle dans Godot ni ralentir la fin de tour (PB1).

## Décision

1. **Le cœur enregistre, Godot rejoue.** `sim-campaign/src/ai_replay.rs` : pendant le tour de
   l'IA, `play_ai_turn` passe par `continue_ai_marches` et `apply_ai_order`, qui, si
   l'enregistrement est actif, gardent chaque `MoveReport` (déplacement, attaque) et chaque
   traversée (`Embark`) sous forme d'`AiMoveRecord`. L'enregistrement ne lit que l'état : le jeu
   est identique avec ou sans (test `ai/tests/ct1_ai_replay.rs`, sauvegardes comparées).
2. **Coût nul par défaut.** L'enregistrement est coupé tant que l'interface ne l'active pas
   (`CampaignSim.set_ai_turn_recording(enabled, notable_radius_km)`, appelé avant chaque fin de
   tour selon le réglage). Coupé (« Masquer »), `end_turn` fait exactement le travail d'avant.
   Actif : deux calculs de vision du joueur (≈ 0,5 ms chacun) et une copie des trajets.
3. **Format** (`CampaignSim.get_ai_turn_moves()`, un dictionnaire par mouvement, dans l'ordre du
   tour : factions par id, marches reprises puis ordres) :

   | clé | type | sens |
   |---|---|---|
   | `sequence` | int | rang dans le tour (0, 1, …) |
   | `army`, `faction` | String | armée IA et sa faction |
   | `path` | PackedVector2Array | pixels carte parcourus, départ compris (≥ 2 points) |
   | `kind` | String | `march`, `stationed`, `siege_started`, `settlement_taken`, `battle`, `landing` |
   | `settlement` | String | colonie atteinte, assiégée, prise ou port de débarquement (`""` sinon) |
   | `target_army`, `target_faction` | String | armée attaquée (ou dont la zone de contrôle a arrêté la marche) |
   | `visible`, `visible_from`, `visible_to` | bool, int, int | partie du trajet vue par le joueur (vision M5a avant **ou** après le tour de l'IA) |
   | `notable`, `priority` | String, int | `battle` (4) contre une armée du joueur, `siege` (3) d'une place qu'il tenait, `player_territory` (2) sur ses terres, `near_player` (1) arrivée à moins de `notable_radius_km` de ses armées ou colonies ; `""` / 0 sinon, et toujours pour ses alliés et vassaux hors bataille et siège |

   Deux mouvements successifs de la même armée qui se prolongent (marche reprise puis nouvel
   ordre) sont fusionnés. Les enregistrements ne sont jamais sauvegardés (`#[serde(skip)]`) et
   sont refaits à chaque fin de tour ; ils sont déterministes (même graine, mêmes trajets).
4. **Godot** (`game/scripts/map/ai_turn_replay.gd`) : entre `end_turn` + `refresh_all` et le
   rapport de saison, replace chaque armée rejouée à son point de départ, l'anime le long de
   `path` (réduit à sa partie vue ± 1 point), et en mode « Suivre » porte la caméra sur les
   `max_followed_moves` mouvements de plus forte priorité avant de la ramener. Réglages
   `map/ai_moves` (Suivre / Montrer / Masquer) et `map/ai_moves_speed` (×1, ×2, ×4) ; Espace
   passe. Mise en scène dans `data/ui/ai_turn_replay.json` (schéma `ai_turn_replay.schema.json`).

## Conséquences

- La visibilité et l'intérêt d'un mouvement sont décidés dans le cœur (règle de brouillard M5a,
  alliances) ; Godot ne fait que filtrer (brouillard coupé : tout est montré) et mettre en scène.
- Une armée vue pendant sa marche mais hors de vue à la fin (et donc sans marqueur) n'est pas
  rejouée ; une armée détruite dans sa bataille n'est pas animée, la caméra se porte sur le lieu.
- `_on_end_turn` devient une coroutine ; sans écran (tests headless), le mode est « Masquer » sauf
  demande explicite (`AiTurnReplay.allow_headless`), si bien que les tests existants restent
  synchrones.
