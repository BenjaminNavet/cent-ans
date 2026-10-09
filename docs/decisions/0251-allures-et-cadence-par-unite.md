# 0251 — Seuils d'allure, cadence nominale par unité, jitter de locomotion

Statut : accepté (2026-10-09)

## Contexte
RX animation : le galop était choisi dès 5,6 m/s pour un nominal de 4,5 m/s (clip lu à x1,24 dès
l'entrée), le trot allait de x0,94 à x1,65 ; le jitter de cadence de ±7 % par soldat contredit
l'asservissement des pieds ; la table de cadence est par clip, pas par unité.

## Décision
- Nominaux recalés (`battle_gore.json: cadence`) : `c_gallop` et `c_charge` 5,8 m/s, famille
  `c_*trot*` 4,4, famille pas/virages 2,2 ; `trot_min_speed` 3,4 (`battle_animation.json`).
  Lecture à l'entrée : galop x1,03, trot x0,86 à x1,27, pas au plus x1,73 (plafond 1,8).
- `cadence_unit` (battle_gore.json) : facteur de foulée par type d'unité multiplié au nominal
  (absent = 1). Valeurs de départ lues sur vitesse et armure des fiches (lourd < 1, léger > 1) ;
  à affiner à l'œil, aucune sonde de distribution des vitesses de bataille n'a été lancée.
- Jitter de cadence : `loco_rate_jitter` 0,03 pour les boucles de locomotion, `idle_rate_jitter`
  0,07 sinon ; la désynchronisation par la phase est conservée. Uniform `rate_jitter`.
- `role_clips` : états `melee` et `routing` pour porte-étendards et musiciens.
- Archers / arbalétriers en mêlée : `push`, `push_shoulder`, `parry`, `guard` (repli sur les
  clips d'épée sur un kit sans eux). Cavalerie : `c_thrust`, virages, `c_rear`, attente.

## Conséquences
- Pas du cheval et chute de cheval restent keyframés : aucune source Muybridge « walk » n'est
  dans le pipeline (AS8b ne couvre que trot et galop) ; reste à faire avec `track_quadruped.py`.
- Valeurs à juger en jeu (voir rapport de lot).
