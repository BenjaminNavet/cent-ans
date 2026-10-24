# 0748 — Budget mémoire du client Bevy

Date : 2026-10-23. Statut : acceptée. Détails et chiffres : `docs/performance-memoire.md`.

## Contexte
Le juge (iter-02, C1) a relevé 5,6 à 7 Go de RSS (campagne) sans budget documenté. Mesure du lot MEM (release, Ultra,
fenêtre masquée) : pic RSS 2,7 Go en campagne, 2,2 Go en bataille ; empreinte physique (`footprint`) ~3,3 Go dont
1,36 Go de graphiques et ~1,3 Go de gros blocs CPU, dont ~0,7 Go libéré mais resté résident.

## Décision
**Amendée par l'ADR 0776** : le budget porte sur l'empreinte physique (`memory.footprint_mib`), campagne 4 000 Mo, bataille 3 500 Mo (cible) ; le pic RSS reste indicatif.

- Budget : pic RSS **3 500 Mo en campagne, 3 000 Mo en bataille**, vérifié par `tools/bevy_bench/perf_guard.sh`
  (clé `memory` de `perf_thresholds.json`, lue dans `memory.peak_rss_mib` des rapports de banc).
- La référence fiable est `footprint` ; le RSS de `ps` est biaisé par le compresseur macOS.
- Sonde permanente : section `memory` du `--dump-state` (`client/src/agent/memory.rs`).
- Essais `malloc_zone_pressure_relief` et cache disque des tableaux de sol : écartés (pas de gain d'empreinte).

## Conséquences
- Aucune réduction de l'empreinte n'est livrée : les gains restants (~100 Mo de copies CPU, décodage des feuilles de
  sol, déchargement campagne/bataille) exigent des mesures sur machine calme (charge > 80 pendant le lot).
- Le garde ne vaut que sur machine calme.
