# ADR 0067 — Direction de la déroute et contagion de moral

Date : 2026-09-26. Statut : accepté. Lot EP10 (suivi : `docs/wip/ep10-deroute-contagion.md`),
renvoyé par SG5 (ADR 0046 § Suite SG5).

## Contexte

Diagnostic de la session de nuit (SG5) sur la crête miroir avec pieux (`sg4_balance`) : la première
déroute est celle d'une aile de cavalerie du défenseur ; ses fuyards refluent le long de la ligne et
des régiments intacts cèdent l'un après l'autre. Deux règles de `sim.rs` en sont la cause :

1. **Contagion aveugle.** Chaque ami en déroute à moins de 120 m retirait 0,4 point de moral par
   seconde (au plus trois), qu'il cède à côté, passe devant ou soit déjà loin derrière.
2. **Fuite latérale.** Un régiment en déroute fuyait « à l'opposé de l'ennemi le plus proche + vers
   son bord ». En bout de ligne, l'ennemi est sur le flanc : la déroute courait à 45° le long de la
   ligne et la brisait régiment par régiment.

À COMPLÉTER.
