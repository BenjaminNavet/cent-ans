# Lot G4 « Bourgogne et Brabant » — état

Branche : `worktree-agent-a734b058e294e9537` (à partir de `main` ac6b7c2). Périmètre : `core/crates/ai`
(alignement), réglages dans `data/`. Écarts visés (`docs/status.md`, tableau G2) : alliance
anglo-bourguignonne jamais conclue ; Brabant rarement anglais.

## Mesure de départ (`century_probe`, graines 1-5, 464 tours, main ac6b7c2)
- Bourg.-Angl. : 0 % partout ; Angleterre dominante 0/0/0/3/0 % (max 3-10 provinces).
- Angl.-Brabant : 0 % partout ; Angl.-Hainaut 0/0/0/0/38 %.
- Guerre FR-EN 53/75/58/55/25 % (moy. 53 %) ; survie 1400 : 4/4 partout.

## Diagnostic
- `TRACE=1 century_probe 464 <graine>` : état de la Bourgogne tous les 5 ans et événements la citant.
- Graine 1 : Montereau (1419) puis « Sceller l'alliance avec Henri V » (1420) se déclenchent : loyauté 49,
  attitude −33 envers la France, +37 envers l'Angleterre… mais la France et l'Angleterre sont en paix et
  l'Angleterre ne domine pas le royaume : aucune défection ; les griefs expirent en 5 ans.

## Points
1. [ ] Grief : un vassal ou allié ulcéré par son patron passe à l'ennemi (ou au prétendant) de celui-ci.
2. [ ] Domination relative (part du royaume) au lieu de 8 provinces.
3. [ ] Réglages dans `data/ai/alignment.json` + schéma.
4. [ ] Brabant 20-60 %.
5. [ ] Tests, mesures avant/après, docs/status.md.

## Prochaine étape
Prototype du grief dans `core/crates/ai/src/alignment.rs`, mesure.
