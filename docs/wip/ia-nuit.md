# IA — nuit du 2026-09-30 : IA performante en campagne et en bataille

Branche `feat/ia` (worktree `../gp-ia`, depuis `main` 6a0d960c9). Mandat du joueur : « tu
travailles sur l'IA du jeu et tu t'assures qu'elle soit performante en campagne ou en bataille »,
autonomie toute la nuit.

« Performante » = les deux sens : l'IA joue bien (bataille : bat une IA naïve à forces égales,
exploite terrain et armes ; campagne : défend, prend les villes vides, ne se ruine pas) et elle
est rapide (tour de campagne < 50 ms par faction, spec M3 ; coût de l'IA par tick de bataille).

## État
- [ ] Mesures de départ (turn_perf, balance_probe, sonde bataille)
- [ ] Diagnostic
- [ ] Correctifs
- [ ] Mesures finales, fusion

## Prochaine étape
Mesures de départ.
