# 0246 — Outil de mesure de campagne officiel

Statut : accepté

## Contexte
Le commit `c2308153d` (SC) a supprimé `century_probe`, `balance_probe`, `ia_quality_probe` et `ai_duel_probe`
(plus de 4 500 lignes d'exemples jetables). Les ADR 0085, 0100, 0148 et plusieurs wip y renvoyaient encore : les
cibles d'équilibrage (guerre FR-EN 55-75 %, révoltes 4-10, banqueroutes) n'étaient plus vérifiables par une commande.

## Décision
Un seul outil maintenu : `core/crates/ai/examples/campaign_probe.rs`.

    cd core && cargo run --release -p ai --example campaign_probe -- --turns 120 --seeds 1,2 [--full] [--json] [--top 5]

Il joue N graines × T tours (IA stratégique pour toutes les factions, la France répondant aux offres comme l'IA) et
sort, en texte ou en JSON : issue et tour, part de tours en guerre FR-EN, guerres actives (moyenne, max) et déclarées,
factions éliminées, révoltes, banqueroutes, revenu/trésor/provinces des K premières factions, plus grosse faction
(fin et pic), temps par tour. `--full` poursuit après l'issue de la partie (conditions de victoire neutralisées pour la
mesure). `CENT_ANS_DATA_DIR` fait jouer une autre copie des données.

## Conséquences
- Toute cible d'équilibrage doit se vérifier avec cet outil ; en ajouter une mesure = l'ajouter ici, pas un nouvel exemple.
- Les anciennes sondes restent consultables par `git show c2308153d^:core/crates/ai/examples/<nom>.rs`.
- Les ADR 0085, 0100, 0148 portent une note de renvoi ; les mesures fines d'alors (EQ4/EQ5/FE8) ne sont pas reprises.
