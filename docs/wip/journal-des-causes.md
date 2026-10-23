# Journal des causes (ADR 0748)

Branche : worktree-agent-a810e54a95d255ac3.

## État
- ADR 0748 écrit. Squelette en cours.

## Plan
1. core/sim-campaign/src/causes.rs : types, journal borné, harvest, migration v9 -> v10.
2. Producteurs : bataille, place prise, banqueroute, révolte, guerre (+ raison IA), alliance rompue, succession.
3. data/ui/causes.fr.json + schéma + test_schemas.py.
4. game-api/src/causes.rs : vues (filtre, top 3, cause d'un événement).
5. client : onglet Causes, fin de bataille, fin de partie, infobulles.

## Prochaine étape
Types + harvest + tests.
