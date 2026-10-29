# ECO-R2a — semonce féodale (boucle juge 2)

Spec : `docs/audits/2026-10-29-economie-politique.md` § R2-a. ADR XXXX.

## État
- Données : `data/rules/feudal.json` bloc `host` + schéma + `HostRules` (data-model).
- Cœur : `sim-campaign/src/feudal/host.rs` (contingent, résolution de saison, ordre `SummonHost`),
  `Army::host_service`, `FeudalState::host_summoned`, garde-fous fusion/scission/garnison.
- IA : `ai::feudal::plan_summon_host`. game-api : `feudal::summon_host`, `DutiesView::host_enemy/host_wait`.
- Client : bouton « Convoquer l'ost » dans la fiche du vassal sélectionné (panneau féodal).
- Tests : `sim-campaign/tests/feudal/eco_r2a.rs`.

## Prochaine étape
Mesure sonde avant/après (avant = `contingent_percent_of_income: 0`), calibrage, seed bots rapide, clippy/tests.
