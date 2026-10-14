# Seed bots — constats retenus par le joueur

Constats des passes seed bots (ADR 0348) que le joueur a choisi de garder. Un constat corrigé reste ici, barré, avec le commit.

## Ouverts

- **Faillites sous la bande** (13/10, passe longue du chantier RE) : `bankruptcies_per_game` vaut en moyenne 21 (bot diplomat), 31 (builder) et 42 (sailor) sur 4 parties chacun, sous la bande [66,75 ; 365]. La bande date d'avant les correctifs de faillite de l'ADR 0350 : probablement périmée. À recalibrer (nouvelle bande depuis une passe longue sur main), sauf si une passe montre que les faillites ont disparu là où elles devraient exister.

- **Tour lent** (13/10) : 5 165 ms pour une limite de 5 000, graine 1, bot random-legal, tour 17, pendant la passe rapide de `fix/bankruptcy`. La machine était très chargée (load ≈ 23), la cause est donc probablement externe. À revérifier sur une machine calme ; si le tour reste au-dessus de la limite, profiler ce tour (`replay`).
