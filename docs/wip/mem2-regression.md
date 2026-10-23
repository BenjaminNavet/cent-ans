# MEM2 : régression de mémoire en campagne

Constat du juge (iter-03, R2) : footprint campagne 10-12 Go (budget 3 500 Mo, ADR 0748).

## État
- Base : main c26909646 (contient fb6b6e23d). Build release en cours.
## Prochaine étape
- Mesurer `footprint` t+8 s / t+30 s ; `vmmap -summary` ; dump `memory`; bissection par fusions.
## Mesures
(à remplir)
