# 0772 — Le compteur « Nuit dans » suit l'heure de départ

## Contexte
Le compteur de bataille affichait toujours 60:00 au départ (`MAX_DURATION - elapsed`), quelle que soit l'heure
(aube, midi, crépuscule), ce qui contredisait le ciel (constat iter-03-art n° 8).

## Décision
`BattleSim::deadline_s()` = temps de bataille jusqu'à la phase « nuit » (20 h 30, à 0,2 min de jour par seconde),
borné à [`MIN_DURATION` 900 s, `MAX_DURATION` 3600 s]. `time_left_s()` et le délai de fin de bataille l'utilisent.
Aube : 60:00 ; midi : 42:30 ; crépuscule : 15:00.

## Conséquences
Les batailles lancées tard sont plus courtes (règle dans `core/`, tests dans `tests/combat/battle.rs`).
