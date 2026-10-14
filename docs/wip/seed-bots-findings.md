# Seed bots — constats retenus par le joueur

Constats des passes seed bots (ADR 0348) que le joueur a choisi de garder. Un constat corrigé reste ici, barré, avec le commit.

## Ouverts

- **Faillites en baisse sur la carte composée** (14/10) : passe longue de `cc/c6` (32 parties), `bankruptcies_per_game` sous la bande de main [66,75 ; 365] pour les 8 bots (moyennes 7 à 47,8). Cause probable : entretien français 8 058 → 4 830 (net France +30 %) après la renormalisation C6. À trancher avant la fusion dans main : rééquilibrer (entretien, revenus) ou recalibrer la bande sur la nouvelle carte.
- **Tour lent** (13/10) : 5 165 ms pour une limite de 5 000, graine 1, bot random-legal, tour 17, pendant la passe rapide de `fix/bankruptcy`. La machine était très chargée (load ≈ 23), la cause est donc probablement externe. À revérifier sur une machine calme ; si le tour reste au-dessus de la limite, profiler ce tour (`replay`).
