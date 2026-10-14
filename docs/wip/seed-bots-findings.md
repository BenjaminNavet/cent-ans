# Seed bots — constats retenus par le joueur

Constats des passes seed bots (ADR 0348) que le joueur a choisi de garder. Un constat corrigé reste ici, barré, avec le commit.

## Ouverts

- ~~**Faillites sous la bande**~~ (13/10 sur main après l'ADR 0350 : 21-42 par partie ; 14/10 sur la carte composée :
  2 à 60 selon le bot, `war_turns_mean` 10,5 pour diplomat ; rapport `core/target/seed-bots/2026-10-14_173759/`).
  Tranché le 14/10 : la bande datait d'avant l'ADR 0350, moins de faillites est l'effet voulu ; bandes
  `bankruptcies_per_game` et `war_turns_mean` recalibrées sur la carte composée par `bands-propose` (autres inchangées).

- **Tour lent** (13/10) : 5 165 ms pour une limite de 5 000, graine 1, bot random-legal, tour 17, pendant la passe rapide de `fix/bankruptcy`. La machine était très chargée (load ≈ 23), la cause est donc probablement externe. À revérifier sur une machine calme ; si le tour reste au-dessus de la limite, profiler ce tour (`replay`).
