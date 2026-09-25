# EQ4 — sonde d'équilibre combinée (EQ1 + EQ2 + EQ3 + DP2 + DF1 + C4/C5 + SG4)

Branche : `worktree-agent-a2b33defc792e3eab` (main 2452ef13).

## État : en cours

- [x] Étendre `century_probe` (révoltes, sièges et leur issue, boule de neige 1437/1453,
      factions éliminées, droit de passage) : tableau « EQ4 — tableau combiné ».
- [x] Mesure de référence (main) : 5 graines × 464 tours × 4 niveaux + `balance_probe` 8 × 200.
- [x] Essai A (Paix de Dieu trouble -8 → -6, piété 2 → 1) : révoltes 3,1 → 5,4 sur les graines
      1-8, mais la référence donne 7,1 sur les graines 9-16 (A : 7,2) ; 16 graines : 5,1 sans,
      6,3 avec. Pas de dérive réelle : **annulé**.
- [x] Essai B (facile : `ai_upkeep_percent` 100 → 90) contre les banqueroutes chroniques
      (Suisses, Gueldre) : 10 graines, 0,44 / fac. / déc., boucles toujours là : **annulé**.
- [ ] main fusionné (SV2 coûts en ressources, SV4) : nouvelle mesure complète en cours.
- [ ] Addendum ADR.

## Référence main (2452ef13)

Normal : guerre FR-EN 60 % [58-61] (5/5 dans 55-75 %), trêves 12,6 [10-15], révoltes
2,8 / 200 tours, banqueroutes 0,20, sièges 430 engagés, 35 % réussis, 1re faction 25 % (1437)
et 23 % (fin), 0,6 faction éliminée, 113 accès militaires accordés, 1210 saisons d'intrusion,
99 casus belli d'intrusion. `balance_probe` : révoltes **3,1** (cible 4-10 ; EQ3 : 4,2),
trouble moyen 20,4, Paix de Dieu dans 73 % des provinces, banqueroutes 0,15.

Facile 56 % (2/5) ; difficile 53 % (1/5) ; très difficile 51 % (2/5), Angleterre jusqu'à 38 %
des provinces.

## Prochaine étape
Tableau final sur main fusionné (`run_final` : normal 10 graines, autres 5, balance 16),
puis rédaction et addendum ADR 0054.
