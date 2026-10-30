# ADR 0816 — IA-PLAN-2 : plans offensifs, rôles `fix` / `wear` / `strike` / `screen`

## Contexte

IA-PLAN-1 (ADR 0809) a posé le planificateur et le plan défensif. Au banc (20 graines), la posture tenue faisait
perdre Crécy (14 -> 9 victoires du défenseur) et Castillon (2 -> 0), et la discipline de cible coûtait des victoires.
IA-PLAN-2 devait ajouter des plans offensifs et corriger ces régressions.

## Décision

- Rôles `fix`, `wear`, `strike`, `screen` ajoutés à `roles.rs`, en données dans `battle_plans.json` (schéma mis à jour).
  `fix` : l'infanterie d'une division qui n'a pas d'ordre d'attaque de l'IA historique attaque la cible de la division.
  `strike` : l'infanterie contourne le flanc de la cible puis charge ; la cavalerie reste à la charge groupée
  (`ai/mass.rs`, ADR 0802). `wear` et `screen` ne donnent pas d'ordre propre (approche et volées historiques).
- Plan *Fixer et déborder* (attaquant) : phases `approach` (wear) -> `provoke` (fix, si personne n'est au contact) -> `fix`
  (avant-garde et bataille fixent, arrière-garde frappe). Sorties par `exit_any`; métrique `engaged` (part des régiments
  en mêlée).
- Le plan défensif garde la posture tenue et la réserve historique (`legacy_reserve`) ; la discipline de cible est un
  réglage de plan (`target_discipline`), désactivé : elle coûtait Crécy et Poitiers.
- Le défaut reste `planner: legacy` : le critère de bascule n'est pas atteint (voir conséquences).

## Conséquences

Banc, 20 graines, victoires de l'attaquant (A) sur `legacy` / `plans` : Crécy 6 / 4 (défenseur 14 / 16, régression
corrigée), Poitiers 8 / 2, Formigny 3 / 2, Azincourt 0 / 0, Auray 0 / 0, Castillon 18 / 20 (défenseur 2 / 0 : non
corrigé, le scénario est déjà perdu par `legacy`). IA contre IA symétrique (`symmetric_7`) : 10 / 4 sur 20, hors de la
bande 40-60 % de la spec. Mesures ablatives : défenseur historique + plan offensif = `legacy` à une graine près ; c'est
la posture tenue du défenseur qui déplace l'équilibre, le plan offensif est neutre. La bascule du défaut est donc
reportée : il faut d'abord rééquilibrer la patience du défenseur (suite possible : IA-PLAN-2b). Non atteints :
`plan_field` < 150 lignes et `battle_ai.json` < 50 clés (la bascule `plans` remplaçant `plan_field` n'est pas faite).
