# 0323 — Ressenti de bataille : ralliement, munitions basses, temps restant, plans de mise en scène

Statut : accepté (lot TW `bfeel`, rapports `docs/wip/tw/bataille-ressenti.md` § 3 top 1-6 et 9,
`docs/wip/tw/bataille-ia-sieges.md` § 3 top 3).

## Contexte
Les critiques relevaient des manques de lisibilité et de ressenti face à Medieval II / Warhammer III :
aucun retour visuel à l'ordre, des ordres d'arrêt/formation/retraite muets, des munitions illisibles
sur le repère, un ralliement sans signal, aucun temps restant avant la nuit, ni ralenti à la chute du
général ni plan de victoire, et un repère flottant sans infobulle.

## Décision
- **Cœur (règles et état exposé).**
  - `rallied` dans `get_units` : vrai pendant `rally.rallied_flag_s` (3 s, `data/rules/battle_morale.json`)
    après la sortie de déroute (`BattleSim::unit_rallied`, dérivé de `rally_timer`, aucun état
    sérialisé nouveau). Alerte typée `rallied` (`AlertKind::Rallied`, importance 35) émise au
    ralliement naturel.
  - `low_ammo` dans `get_units` (`UnitStatus.low_ammo`) : tireur sous `status.low_ammo_ratio` (0,25,
    `data/rules/unit_modes.json`) de ses munitions pleines.
  - `BattleSim::time_left_s` (`MAX_DURATION` - temps écoulé), pont `get_time_left_s`. À la nuit, le
    défenseur l'emporte (`BattleEnd::Nightfall`, inchangé).
  - Rien de tout cela n'entre dans `state_digest` ni dans le rejeu.
- **Godot (rendu seulement, réglages dans `data/fx/battle_staging.json`)** : `order_flash` (anneau au
  sol 0,6 s, rouge sur l'ennemi attaqué, couleur du camp sinon, `BattlePathPreview.flash_order`),
  `general_fall_slowmo` (échelle 0,35 pendant 1,2 s, coupé en IA contre IA, en rejeu et par le réglage
  « Ralenti du plan »), `victory_shot` (orbite 4 s sur le général vainqueur avant l'écran de fin,
  passable, jamais en headless/IA contre IA), `time_warnings_s` (300 s et 60 s : bandeau).
  Pastilles `low_ammo` et `rallied` dans `state_badges` ; infobulle du repère = celle de la carte
  d'unité (`UnitCard.tooltip_live`). Barks `hold` (halt), `formation`, `retreat` (withdraw) et `rally`
  dans `data/voice/barks.json`.

## Conséquences
- Les clips voix des quatre situations ne sont pas générés (aucune dépense) : la réplique est décidée
  puis reste silencieuse (`BattleVoices.play` sans flux) ; liste des clips dans `docs/wip/tw/bfeel.md`.
  Le plafond de 40 lignes par langue (test `test_voice.py`) limite le corpus à 5 lignes ajoutées en
  fr et en ; les langues régionales se replient sur fr/en.
- Anneau d'ordre : rayon 1 à 3,2 m (et non 0,4 à 1,2 m du rapport, trop petit à la distance de jeu).
- Non fait : recentrage doux de la caméra à la chute du général ; bark `flanked`.
