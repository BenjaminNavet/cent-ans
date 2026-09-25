# SG1 — Batailles de siège à la Total War

Branche `worktree-agent-a806226fd4b8b61a9`. Backlog `docs/audit/backlog-tw.md` (Bataille : sièges).

## Objectif
Assauts spectaculaires et lisibles : échelles dressées et escaladées, beffrois accostés (pont-levis
abaissé), bélier sous manteau qui frappe en rythme, trébuchets et bombardes (projectiles, traînée,
éclats de pierre), défenseurs sur le chemin de ronde, combat sur le mur, huile bouillante, porte qui
cède, garnison qui se replie sur la place.

## Architecture
- Cœur (`core/crates/sim-battle`) :
  - `src/siege_fx.rs` : `SiegeFx { time, kind }`, événements pour le rendu (tir d'engin avec point
    d'impact, coup de bélier, volée d'une tour, échelles dressées, beffroi accosté/décroché, prise du
    rempart, huile bouillante, porte enfoncée, brèche, repli de la garnison). Aucun tirage aléatoire
    (hachage entier) : déterminisme intact.
  - `src/sim/siege_assault.rs` : file d'événements, coups du bélier toutes les `RAM_PERIOD` = 3 s
    (même dégât moyen qu'avant), **huile bouillante** (nouvelle règle : un pot toutes les 20 s si un
    défenseur garde la porte, sur les assaillants à moins de 10 m de sa face ; armure à moitié
    efficace ; bélier protégé par ses peaux), transitions (porte, brèche, beffroi).
  - `ai.rs` (`plan_siege_defence`) : au plus 2 régiments par ouverture la bouchent, le reste de
    l'infanterie de la garnison se regroupe sur la place.
- Pont : `BattleSim.get_siege_events()`, `get_siege().ram_period / oil_period`.
- Godot : fichier séparé `game/scripts/battle/siege_assault_fx.gd` (ne touche pas aux maisons de
  BR1 dans `battle_siege.gd`).

## État
- [x] Squelette cœur + pont (compile).
- [ ] Tests `sim-battle/tests/sg1.rs`.
- [ ] Rendu : engins animés, projectiles d'engins vers la muraille, éclats, porte qui éclate, huile.
- [ ] Échelles contre le mur + soldats sur les échelles (clip `climb`).
- [ ] FPS avant/après, captures `docs/audit/captures/sg1/`.

## Prochaine étape
Tests du cœur, puis `siege_assault_fx.gd`.
