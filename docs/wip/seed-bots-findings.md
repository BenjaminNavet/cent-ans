# Seed bots — constats retenus par le joueur

Constats des passes seed bots (ADR 0348) que le joueur a choisi de garder. Un constat corrigé reste ici, barré, avec le commit.

## Ouverts

- ~~**Faillites en baisse sur la carte composée**~~ (14/10, tranché) : passe longue de `cc-carte-composee` après CS
  (32 parties, rapport `core/target/seed-bots/2026-10-14_173759/`) : 2 à 60 faillites par partie selon le bot,
  `war_turns_mean` 10,5 pour diplomat. Moins de faillites = l'effet voulu (ADR 0350, entretien renormalisé C6) ;
  bandes `bankruptcies_per_game` et `war_turns_mean` recalibrées sur la carte composée par `bands-propose`
  (autres bandes inchangées).
- **Tour lent** (13/10) : 5 165 ms pour une limite de 5 000, graine 1, bot random-legal, tour 17, pendant la passe rapide de `fix/bankruptcy`. La machine était très chargée (load ≈ 23), la cause est donc probablement externe. À revérifier sur une machine calme ; si le tour reste au-dessus de la limite, profiler ce tour (`replay`).
