# OMR R1 — coût de planification IA par tour (carte OM)

Branche `feat/omr-r1`, worktree `../gp-omr-r1`. Plan : `docs/wip/omr.md`.

## Objectif
Tour de jeu OM ≈ 1,07 s de planification IA (177 factions) contre 0,23 s sur l'ancienne carte ;
cible ≤ 0,45 s, décisions de l'IA inchangées (empreinte `turn_digest` identique à graine égale).

## Méthode
- Cible cargo partagée `core/target`, profils `r1` (dev, debug=0) et `r1rel` (release, debug=0).
- Mesure : `turn_perf 10 1 1` (release sans debug) ; égalité : `turn_digest` avant/après.
- Profil : `sample` sur `turn_perf`.

## État
- [ ] Mesure de référence + empreinte de référence
- [ ] Profil
- [ ] Caches (tests d'égalité ancien = nouveau)
- [ ] Mesure après

## Mesures

## Prochaine étape
Construire `turn_perf`/`turn_digest` en `r1rel`, mesurer, profiler.
