# ADR 0800 — BANC-1 : banc d'équilibre des batailles

## Contexte

L'audit de la bataille (`docs/audits/2026-10-29-bataille.md`, C2/C3) constate que l'équilibre n'est mesuré nulle part de
façon unique : les bandes d'issue sont des tests à graine éparse (ep7, ep9, matchups), l'exemple `balance_probe` a disparu
et l'auto-résolution n'est plus calibrée (ADR 0013, 0792). Chaque refonte rejouait donc l'histoire des ADR 0184/0792.

## Décision

- Un exemple Rust `core/crates/sim-battle/examples/battle_bench.rs` rejoue une matrice déclarée dans données
  `data/rules/battle_bench.json` (schéma `data/schemas/battle_bench.schema.json`) : duels 1 contre 1 des 14 types
  principaux, 6 armées de campagne types, les 6 cartes historiques (Crécy, Poitiers, Azincourt, Formigny, Castillon,
  Auray), 3 sièges ; modes `ai_vs_ai`, `passive_attacker`, `passive_defender`, `blob`, `flank`.
- Le banc ne contient aucune règle de jeu : il pilote `BattleSim` et mesure (taux de victoire, pertes médianes, t du
  premier contact, t de la première déroute et pertes à ce moment, durée, part de munitions tirées, régiments en fuite et
  détruits).
- Sortie `out/bench/<date>.json` (octets stables : arrondis fixes, pas d'horloge, ordre des lignes fixe, indépendant du
  nombre de fils) et résumé texte. Le résultat d'une passe de référence est consigné dans `docs/bench/`.
- Commande : `tools/heavy.sh cargo run --release -p sim-battle --example battle_bench` (`--smoke`, `--filter`,
  `--threads`, `--matrix`, `--out`).
- Tests (`cargo test -p sim-battle --examples`) : `bench_smoke`, `bench_is_deterministic`, `matrix_is_consistent`.

## Conséquences

- Toute retouche d'équilibre se juge sur une passe du banc comparée à la référence.
- BANC-2 (fixture d'auto-résolution + `data_hash`) et BANC-3 (bandes d'issue lues du banc) s'appuient sur ce JSON.
- Les modes `blob` et `flank` reprennent ceux de la sonde d'audit (`out/audit/bataille/probe`).
- Le banc mesure seulement : aucune règle de combat ni d'IA n'est touchée par ce lot.

## Matrice de référence (29/10/2026, `docs/bench/reference-2026-10-29.{json,txt}`)

133 lignes, 29 s en release à 4 fils. Deux passes (4 fils, 2 fils) : JSON identique octet pour octet.
Victoires de l'**attaquant** de la carte (12 graines ; sièges 8 ; duels 6), IA contre IA :

- Armées : FR/EN plaine 5/12, FR/EN collines 5/12, EN attaque FR 10/12, symétrique 7/12, grande bataille 3/12,
  4 chevaliers contre 20 milices 3/12 (la bande de l'audit, « chevaliers 30-80 % », tient tout juste).
- Cartes historiques : Crécy 2/12, Poitiers 4/12, Azincourt 0/12, Formigny 1/12, Castillon 11/12, Auray 0/12
  (qui attaque dépend de la carte ; lire les camps dans `data/battle_maps/`).
- Sièges : enceinte 1 : 8/8, enceinte 2 : 8/8, enceinte 3 : 0/8 (à l'inactivité du défenseur : 7/8, 5/8, 0/8).
- Duels : 91 paires, l'attaquant (premier type de la paire) gagne en moyenne 39 % ; 27 paires sans aucun contact
  (cavalerie lourde ou milices contre le reste : durée de bataille atteinte sans mêlée), 9 sans aucune perte des deux côtés
  (chevaliers et sergents montés contre milices, hobelars, Gallois, goedendag, routiers) : le camp « vainqueur » l'est alors
  par défaut. Ce sont les « batailles où personne ne touche personne » de l'audit (M2), désormais mesurées.
- Modes de joueur : un attaquant inactif perd tout (FR/EN plaine 0/12, symétrique 0/12) alors qu'un défenseur inactif
  perd presque toujours (symétrique : attaquant 12/12). « Tous à l'attaque » est pire que l'IA (FR/EN plaine 0/12 avec 29 %
  de pertes propres contre 4 %). La cavalerie en flanc est à peu près neutre (6/12, 8/12).
