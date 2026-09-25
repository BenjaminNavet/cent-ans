# SG4 — IA d'assaut de siège et équilibre défenseur / attaquant à l'échelle épique

Branche `worktree-agent-a65913a9ab64f3915` (worktree `agent-af1d6ff622ee45890`). Suite de SG3
(`docs/wip/sg3-sieges.md`), ADR 0023 (SG2, SG3), 0031-0034 (EP), 0046 (R4).

## Objectifs
1. IA d'assaut (`plan_siege_attack`, `core/crates/sim-battle/src/ai.rs`) : relève de l'équipage
   du bélier, échelles sur plusieurs pans, infanterie par la brèche ou la porte dès l'ouverture,
   pas d'unité inactive au pied du mur, retraite si l'assaut est perdu. Cible : < 20 % de nuls à
   Avignon avec engins (sonde `sg3_assault_probe`, `ENGINES=1`).
2. Défenseur R4 à forces égales à l'échelle épique : matrice attaquant / défenseur (plat, crête,
   avec et sans pieux) sur 10 graines ; correction par les données de R4.

Consigne de coordination : ne pas changer la signature de `ai.rs::defensive_ground` ni des
modificateurs de terrain (EP6 en cours).

## État
- [ ] Mesure de départ (sonde SG3 avec et sans engins).
- [ ] Diagnostic des nuls d'Avignon.
- [ ] IA d'assaut.
- [ ] Matrice R4 et réglage par les données.

## Prochaine étape
Mesure de départ en cours.
