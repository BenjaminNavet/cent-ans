# 0404 — Seuil de perf des lots de qualité visuelle (chantier VQ)

Statut : accepté (17/10, chantier VQ).

## Contexte
Le joueur demande plus de qualité visuelle sans passer sous 40 images/s. Mesures de départ (bm-perf4, release, Préréglage « high ») : bataille calme 9 ms/image ; campagne panoramique d=30 p50 12,6, p95 24, p99 26 ms. La bataille a de la marge, la campagne n'en a pas sur ses pires images. La machine est souvent chargée par d'autres sessions (charge ~90), ce qui rend les chiffres absolus bruités.

## Décision
1. Seuil : **p95 ≤ 25 ms** (40 i/s sur les pires images courantes) au préréglage « high » à 1280×800 et à la résolution de la fenêtre du joueur, sur le banc carte (`tools/bevy_bench/bench.sh`, pan + descente) et sur le banc bataille (`--bench-battle`, lot vq-bench).
2. Chaque lot qui ajoute du coût de rendu le mesure en **A/B alterné dans la même session** (interrupteur `CENT_ANS_PERF_OFF` ou réglage, 3 paires A/B alternées) et donne le surcoût en ms au p50 et au p95. Le lot est refusé si le p95 dépasse 25 ms, ou s'il ajoute plus de 2 ms au p95 de la campagne tant que le lot vq-cperf n'a pas libéré la marge.
3. Tout nouvel effet coûteux a un réglage dans `data/fx/render_quality.json` (coupé ou réduit en « low »/« medium ») : la qualité monte d'abord en « high »/« ultra ».
4. Banc final sur machine calme (charge < 10) avant clôture du chantier.

## Conséquences
- La campagne passe d'abord par un lot de perf (vq-cperf) avant les effets coûteux de près.
- Les chiffres bruités d'une machine chargée servent seulement en relatif (A/B), jamais pour le seuil.
