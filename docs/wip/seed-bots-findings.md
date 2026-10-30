# Seed bots — constats retenus par le joueur

Constats des passes seed bots (ADR 0348) que le joueur a choisi de garder. Un constat corrigé reste ici, barré, avec le commit.

## Ouverts

- ~~**Faillites sous la bande**~~ (13/10 sur main après l'ADR 0350 : 21-42 par partie ; 14/10 sur la carte composée :
  2 à 60 selon le bot, `war_turns_mean` 10,5 pour diplomat ; rapport `core/target/seed-bots/2026-10-14_173759/`).
  Tranché le 14/10 : la bande datait d'avant l'ADR 0350, moins de faillites est l'effet voulu ; bandes
  `bankruptcies_per_game` et `war_turns_mean` recalibrées sur la carte composée par `bands-propose` (autres inchangées).

- **Tour lent** (13/10) : 5 165 ms pour une limite de 5 000, graine 1, bot random-legal, tour 17, pendant la passe rapide de `fix/bankruptcy`. La machine était très chargée (load ≈ 23), la cause est donc probablement externe. À revérifier sur une machine calme ; si le tour reste au-dessus de la limite, profiler ce tour (`replay`).

- **Score de règne sous la bande** (30/10, passes longues sur main après FUSION-A et FUSION-B, valeurs identiques ; rapport
  `out/seed-long-postB/seed-long.log`) : `reign_score` 0,75 à 3,5 en moyenne pour tous les bots, bande [6 ; 40]. Priorité.
  Attendu en hausse avec CAMP-V (verbes de maison) et CAMP-S (crises à horloge) ; revérifier à la passe longue suivante, sinon lot dédié.

- **Banqueroutes et révoltes en excès** (30/10, même passe) : aggressor 29,5 banqueroutes (bande [0 ; 25]), 20,5 révoltes
  ([0 ; 10]), 13 soulèvements ([0 ; 8]), 33 alertes de troubles ([0 ; 20]), trésor final −4 471 ; random-legal 28 banqueroutes,
  11 révoltes ; sailor 13 révoltes. Économie en pause (décision du 30/10) : à traiter à la reprise des lots économiques.

- **Provinces perdues, prises rebelles** (30/10, même passe) : `provinces_controlled_end` 0,75 à 1 (bande [3 ; 16]) pour
  aggressor, random-legal et sailor ; `rebel_takeovers_per_game` 4,25 à 7,5 (bande [0 ; 4]). Lié aux révoltes ci-dessus.
