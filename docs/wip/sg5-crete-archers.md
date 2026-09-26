# SG5 — les tireurs et la cavalerie du défenseur ne rompent plus leur propre ligne sur la crête

Branche `worktree-agent-add3a9bbd7d238220`. Suite de SG4 (`docs/wip/sg4-assaut.md`), ADR 0046
(§ Suite SG4, **§ Suite SG5**), ADR 0056 (§ EP9b).

## État : terminé, à fusionner par l'orchestrateur

- [x] `git merge main` (EP9b inclus).
- [x] Mesure de départ `sg4_balance` (10 graines) : identique à ADR 0056 § EP9b.
- [x] Diagnostic (sondes `SG4_ROUTS`, `SG4_SHOTS`, `SG4_HORSE=a..b` ajoutées à `sg4_balance`).
- [x] Correctif : ligne d'une armée sur sa crête reculée de 120 m sur la contre-pente
  (`data/rules/battle_crest_defence.json`, schéma, `crest::CrestDefenceRules`, `ai.rs::plan_field`),
  seulement si la ligne compte au moins 8 régiments.
- [x] ADR 0046 § Suite SG5 (mesures, variantes écartées, points ouverts).
- [x] fmt, clippy, cargo test, sonde SG3, `survey_crecy`, pytest, build.sh, import, smoke.

## Mesures (attaquant / défenseur / nuls, 60 régiments, graines 1-10)

| Terrain | Pieux | Départ | SG5 |
|---|---|---|---|
| plat | oui | 3 / 7 / 0 | 3 / 7 / 0 |
| plat | non | 7 / 3 / 0 | 7 / 3 / 0 |
| crête | oui | 6 / 4 / 0 | 0 / 10 / 0 |
| crête | non | 1 / 9 / 0 | 0 / 10 / 0 |
| plaine générée | oui | 2 / 8 / 0 | 1 / 9 / 0 |
| plaine générée | non | 6 / 4 / 0 | 6 / 4 / 0 |
| comme ep1_scale | oui | 6 / 4 / 0 | 5 / 5 / 0 |

Sonde élargie (crête, 40-80 régiments, 20 graines) : 55/160 → 6/160 victoires de l'attaquant.

## Note pour la session épique (contagion, non modifiée)
La contagion compte tout ami en déroute à moins de 120 m (devant, derrière ou le long de la
ligne) ; une déroute fuit « loin de l'ennemi le plus proche + vers son bord » : à l'extrémité
d'une ligne l'ennemi est sur le flanc et la déroute court le long de la ligne (cascade observée :
régiments intacts, ennemi à 150-200 m, qui cèdent l'un après l'autre). Pistes : fuite orientée
vers l'arrière ; contagion réduite pour un ami en déroute derrière le régiment.

## Note pour EP7
Les régiments postés par un scénario (`hold`) doivent être exclus du recul de ligne SG5 à la
fusion (`scenario_post(i)`), même si le filtre de laisse rejette déjà ces ordres.
