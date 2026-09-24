# C7a — repli du perdant, IA sur les colonies, équilibrage 50 tours

Spec : `docs/design/2026-09-24-echelle-colonies.md` (§ 4.3-4.5, § 7), suite de C4
(`docs/wip/c4-core-settlements.md`). Branche : `worktree-agent-a2e37f1c8b64c0bf2` (depuis `main` cda86a9).
Périmètre : `core/` et `data/` uniquement.

## État

- [x] Repli du perdant : règle C7a (`movement::retreat_target`), réglages `data/settlements/rules.json`
  § `retreat` (+ schéma), tests `sim-campaign/tests/c7a_retreat.rs`.
- [x] Sonde `core/crates/ai/examples/settlements_probe.rs` (50 tours × 8 graines, ~3,5 s par graine en release).
- [ ] IA sur les colonies (`ai_minimal`, `crates/ai`) : reprise des colonies perdues, garnisons
  frontalières, forteresses de niveau 4.
- [ ] Équilibrage (paramètres `data/`), mesures avant / après ci-dessous.

## Règle de repli (décision)

Ordre, déterministe (égalités départagées par l'id de colonie) :
1. colonie amie (à soi ou à un allié) sans armée ennemie, la plus proche sur le graphe, dans un rayon de
   `friendly_radius_steps` = 2 pas, par un chemin qui ne traverse aucune place ennemie ;
2. sinon colonie non tenue par un ennemi (neutre) sans armée ennemie dans un rayon de
   `neutral_radius_steps` = 1 pas ; les traînards coûtent `neutral_loss_percent` = 10 % ;
3. sinon **débandade** : `rout_loss_percent` = 50 % de pertes ; les survivants rejoignent la place amie la plus
   proche à toute distance (chemin sans place ennemie) si l'armée garde au moins
   `rout_dissolve_below_percent` = 30 % de ses effectifs, sinon elle se disperse (le général s'échappe).

Un attaquant battu revient toujours d'où il venait, sauf si une armée ennemie y est entre-temps.
Justification : une armée battue loin de ses places et cernée se dispersait (fuite de l'ost de Philippe VI
après Crécy, débandade après Poitiers, compagnies dispersées) ; une armée proche de ses places s'y
réfugiait ; le passage en terre neutre (Empire, Bretagne neutre) était courant mais coûtait des traînards.

## Portée d'une saison (demande C5 / orchestrateur)

Sonde : `cargo run --release -p ai --example reach_probe` (armée principale, départ 1337 ; les places
ennemies arrêtent la marche). Réglages `movement.season_scale` = 0,5 et `movement.road_cost_factor` = 0,75
(les arêtes routières du graphe C3, cuites à 0,5, sont remises à 0,75 au chargement).

| départ | saison | avant : points / colonies / provinces / étapes méd.-max | après |
|---|---|---|---|
| Paris (France) | été | 420 / 233 / 57 / 9-16 (Villeneuve-sur-Lot, 14 étapes) | 210 / 62 / 16 / 5-8 (Vaucouleurs) |
| Paris | hiver | 280 / 162 / 37 / 7-14 | 140 / 24 / 4 / 3-5 |
| Londres (Angl.) | été | 420 / 65 / 15 / 5-10 | 210 / 34 / 7 / 3-6 |
| Bordeaux (Angl.) | été | 420 / 48 / 14 / 3-6 (Lleida) | 210 / 34 / 9 / 3-5 (Lusignan) |
| Bordeaux, sans ennemis | été | 420 / 219 / 50 / 8-18 (Hesdin) | 210 / 48 / 11 / 3-5 |

Une saison couvre ~1,5 pas de province (210 km de plaine), ~2 sur route. `PLANNING_RANGE` de l'IA passe de
8 à 5 pas et `OFFENSIVE_RANGE` d'`ai_minimal` de 6 à 4 (objectifs à 3 saisons au plus).

## Mesures

Sonde : `cargo run --release -p ai --example settlements_probe -- 50 1 2 3 4 5 6 7 8`.

(à compléter)

## Prochaine étape

Équilibrage économique (entretien des bâtiments hors cité, garnisons de départ), mesures après, fusion de
main (C5) et test Godot `c5_settlements_ui_test.gd`.
