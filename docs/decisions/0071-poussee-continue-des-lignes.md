# ADR 0071 — Poussée continue des lignes en mêlée

Date : 2026-09-26. Statut : accepté. Lot EP11 (suivi : `docs/wip/ep11-poussee-lignes.md`),
backlog TW (`docs/audit/backlog-tw.md`, « poussée des lignes, formations qui se déforment au
contact, bords de formation qui s'enroulent »).

## Contexte

Jusqu'ici, le contact entre deux régiments ne produisait que des effets discrets : un impact de
charge (ADR 0022) puis des coups, régiments immobiles l'un contre l'autre jusqu'à la déroute.
Rien ne bougeait au contact : pas de ligne qui cède du terrain, pas de front qui se bombe, pas de
régiment écrasé contre un obstacle, pas de files qui débordent un adversaire plus étroit.
Contraintes : règle au cœur, déterministe (le rejeu EP13 re-simule la bataille), tenue à
15 000 soldats (EP1), batailles de référence dans leurs fourchettes (EP7, EP9, EP9b, EP10).

## Décision

Règles dans `data/rules/battle_push.json` (schéma `battle_push_rules.schema.json`, pytest
`tools/tests/test_battle_push_schema.py`), code dans `sim-battle/src/push.rs` (règles, pression,
forme, déformation des figurines) et `sim-battle/src/sim/push.rs` (`BattleSim::resolve_push`,
appelé une fois par pas juste avant les coups de mêlée). Branchements dans `sim.rs` limités à
l'appel, au champ `push_rules` et à un facteur dans `melee_damage`. Batailles rangées seulement :
un siège garde sa mêlée fixe.

1. **Pression.** Chaque régiment au contact pousse (`drive`) : poids par homme (1 + 0,01 × armure,
   ×2,2 à cheval) × savoir-faire (0,5 + 0,01 × mêlée) × √rangs (au plus 8) × fraîcheur
   (1 − fatigue/300, au moins 0,4) × moral (0,5 + moral/200) × élan de charge (1 + charge/100
   tant qu'il dure) × piques contre cavaliers (×2,5). Il tient (`resistance`) avec la même
   pression, multipliée par un carré (×1,6) ou des pieux plantés (×10 : les pieux font barrière,
   on ne repousse pas des archers derrière leurs pieux). Carré et pieux tiennent, ils ne poussent
   pas.
2. **Recul continu.** L'écart relatif (D − R)/(D + R) donne la vitesse de recul du plus faible :
   rien sous 0,15 (bande morte : deux lignes égales se tiennent), puis un lissage jusqu'à
   0,45 m/s à 0,45. Hommes d'armes à pied contre milice : 2,2 m par 10 s. Le vainqueur suit
   (il reste au contact), sauf s'il est aux pieux ou engagé par un second ennemi : il ne s'enfonce
   pas seul dans la ligne adverse pour s'y faire prendre de flanc.
3. **Compression.** Un régiment poussé qui ne peut reculer (bord du champ, mur, maison, eau
   profonde, régiment ami à moins de 1,2 m derrière, écart mesuré par axes séparateurs) se
   comprime (0 à 1, +0,1/s à pleine poussée, −0,1/s libre) : il porte 25 % de coups en moins,
   en reçoit 20 % de plus et perd 0,25 point de moral par seconde à compression 1.
4. **Recul et moral.** Céder du terrain ébranle : 0,25 point de moral par seconde à pleine
   vitesse de recul.
5. **Enroulement.** Quand un régiment déborde de plus de 3 m, d'un côté, un adversaire plus étroit
   placé devant lui, ses files débordantes pivotent autour du coin de l'adversaire (0 à 1 en
   ~17 s, jusqu'à 70°) et frappent ses flancs : coups × (1 + 0,15 × enroulement moyen).
6. **Forme du front (rendu).** `soldier_poses` déforme les figurines de chaque régiment :
   bombement parabolique centré sur le point de contact (0,6 m de mêlée + 5 m par m/s de
   poussée, au plus 2,5 m ; celui qui recule se creuse d'autant, le dernier rang garde 35 %),
   rangs serrés d'un régiment comprimé (jusqu'à −30 % de profondeur), files enroulées tournées vers
   l'adversaire. Godot ne fait qu'afficher le tampon `get_soldier_buffer` ; `get_units` expose
   `push_speed`, `compression` et `ground_lost`.

État par régiment : `Unit::push` (`PushShape`), sérialisé (`serde(default)`) ; aucune itération
sur table de hachage, aucun tirage : la poussée est strictement déterministe.

## Réglage

La mêlée étant chaotique (un mètre de plus change l'issue de graines entières), les paramètres ont
été choisis par une recherche aléatoire (≈ 90 jeux de paramètres) sur trois critères à la fois :
EP7 (graines 1-20 dans les fourchettes, 1-30 pour la tendance), ep9b miroir (graines 1-10 dans
3-7, 1-30 pour la tendance) et `ai::ai_beats_a_passive_ai_at_equal_forces`. Trois constats :

- sans barrière des pieux (×1,5), les chevaliers repoussaient les archers d'Azincourt et le suivi
  les amenait dans le flanc des hommes d'armes anglais : Azincourt 20/20 ;
- enlever les pieux au régiment repoussé (idée écartée) livrait Crécy aux Français (0/20) ;
- un recul ou une compression trop punitifs pour le moral (0,5/s chacun) faisaient céder la milice
  dans `ai` (le camp passif gagnait).

## Mesures (avant = poussée coupée, même code ; après = règles livrées)

Mac M4 Pro, machine partagée. « Avant » : mêmes binaires, poussée coupée (`max_speed_mps` et
`grow_per_second` à 0 : `resolve_push` ne s'exécute pas, facteur de mêlée 1) — identique à main.

| Mesure | Avant | Après | Fourchette du test |
|---|---|---|---|
| Laboratoire hommes d'armes contre milice (`ep11_push`) | 0 m | 2,2 m / 10 s, 13,4 m en 60 s | 2-9 m en 20 s |
| Crécy, victoires anglaises (`ep7_historical`, graines 1-20 / 1-30) | 16/20, 26/30 | 18/20, 25/30 | 14-19 |
| Azincourt | 19/20, 29/30 | 18/20, 28/30 | 14-19 |
| Poitiers | 12/20, 18/30 | 16/20, 20/30 | 11-18 |
| Durées EP7 min/méd/max (s) : Crécy, Azincourt, Poitiers | 814/848/1088, 520/849/905, 587/1123/1313 | 827/869/1154, 520/831/885, 609/1074/1238 | — |
| `ep9b_duel` miroir plat sans pieux, attaquant (graines 1-10 / 1-30) | 4/10, 7/30 | 4/10, 5/30 | 3-7 sur 10 |
| `ep9_decisive::survey_crecy` (Crécy-like, 12 graines) | 12/12 anglais | 11/12 anglais | majorité |
| `ep9_decisive::survey` (27 lignes × 12 graines) | toutes finies, max 1161 s | toutes finies, max 1161 s | < 30 min |
| Petite bataille B6, graines 0-63 (`ep10_rout::probe_small_battle`) | France 34/64 | France 31/64 | — |
| `ep10_rout` (aile en déroute par le flanc), régiments de la ligne qui cèdent | 0/7 | 0/7 | ≤ 1 |
| Coût du pas, 60 × 120 par camp IA contre IA (`ep11_push::probe_step_cost`, release) | 0,121 ms (pire fenêtre 0,166) | 0,115 ms (0,150) | — |
| Coût du pas, 120 régiments en mêlée d'un bout à l'autre (même sonde) | 0,104 ms | 0,120 ms | — |
| Banc Godot `tools/bench_ep1.sh --units=63` (13 900 soldats, bibliothèque debug), i/s moy. / p95 ms à 240 s | 36,8 · 37,2 / 29,5 · 29,0 | 37,0 · 34,6 / 29,6 · 33,3 | ≥ 30 |
| idem à 450 s (mêlées engagées) | 28,6 / 35,5 | 29,9 / 33,3 | — |

Lecture : le coût du pas reste de l'ordre du dixième de milliseconde (+0,016 ms dans le pire cas,
120 régiments au contact), invisible au banc (les écarts sont dans le bruit de la machine
partagée ; le banc plafonne à ~37 i/s ce matin avec ou sans la poussée). La déformation des
figurines (`deform_figures`) ne coûte rien hors mêlée (retour immédiat) et une rotation par
figurine en mêlée.

## Captures

`game/tests/ep11_push_shot.gd` (vraie simulation, vrai rendu ; `--scene=line|wrap|edge`), hommes
d'armes à pied (bleu) contre milice urbaine (rouge), dans `docs/img/ep11/` :
`ep11_top_line_t1.jpg`, `_t10.jpg`, `_t25.jpg` (vue de dessus 1, 10 et 25 s après le contact :
front bombé des hommes d'armes, front creusé de la milice qui recule de 3,8 m, ailes de la milice
plus large enroulées autour d'eux), `ep11_line.jpg` (même scène, vue oblique),
`ep11_top_wrap.jpg` (milice en colonne : les files débordantes des hommes d'armes l'enveloppent),
`ep11_top_edge.jpg` (milice adossée au bord du champ, compression 0,78 : rangs serrés).
Sans affichage, le script sert de test (contact, recul en `line`, compression en `edge`).

## Conséquences

- Empreintes `b6.rs` recalculées (graine 3 repasse aux Anglais à 312 s ; graine 11 même
  vainqueur, 177 s). Sur les graines 0-63 de la petite bataille B6, la France gagne 31/64 au lieu
  de 34/64.
- Une ligne lourde recule une ligne légère ; une colonne étroite (√8 rangs) tient mieux qu'une
  ligne mince ; un régiment collé à une seconde ligne, à un mur, à l'eau ou au bord du champ est
  plus fragile au contact (compression) : laisser un intervalle derrière une ligne engagée.
- Un ami au repos derrière le régiment poussé, mais décalé de côté, est souvent repoussé par la
  séparation amicale (F5a, test d'écart par les centres, plus pessimiste que les axes séparateurs
  d'EP11) avant d'arrêter le recul : les deux lignes reculent ensemble au lieu de se comprimer.
  La compression contre un ami se produit quand il est aligné ou qu'il ne peut être poussé
  (engagé, en marche).
- Limites : pas de poussée en siège ; le point de contact est le centre de l'adversaire (pas la
  vraie surface de contact) ; la déformation est un rendu, les rectangles de contact et de
  collision restent ceux de la formation ; un régiment qui recule garde son orientation.
- Suites possibles : dépense de fatigue à la poussée, poussée en siège (brèches, portes), IA qui
  évite de coller une seconde ligne à la première.
