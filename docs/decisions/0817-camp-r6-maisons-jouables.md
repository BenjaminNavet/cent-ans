# 0817 — CAMP-R6 : huit maisons jouables

Statut : accepté (décision du joueur, 29/10).

## Contexte

L'audit `docs/audits/2026-10-29-campagne.md` (§ R6) recommande de recentrer le jeu : 139 factions étaient `playable`,
dont la plupart sans ambitions ni verbes écrits (ADR 0805 : trois jeux écrits seulement, plus `default`).

## Décision

1. `playable: true` pour huit maisons : France, Angleterre, Bourgogne, Bretagne, Navarre, Écosse, Flandre, Castille.
   Toutes les autres factions restent dans le monde (armée, trésor, IA, diplomatie) mais ne sont pas sélectionnables.
2. Le drapeau reste le seul interrupteur : écran de choix de faction, fiches de départ, bataille personnalisée
   (les deux camps), codex (« jouable ») le lisent. Aucun schéma modifié (`playable` existait).
3. Compatibilité : la construction d'une campagne (`CampaignState::new`, chargement de sauvegarde, sondes, tests)
   n'a jamais refusé une faction non jouable ; cela ne change pas. Une partie qui joue une faction non jouable reste
   possible par l'API (IA, tests), seule l'interface ne l'offre plus.
4. Effet de bord assumé : les règles qui traitent « jouable » comme « maison du joueur potentielle » (réseau d'agents
   des maisons pauvres, `agents/ai.rs`) ne s'appliquent plus aux autres maisons ; elles suivent le chemin IA ordinaire.

## Conséquences

- Les cinq maisons nouvelles (Bretagne, Navarre, Écosse, Flandre, Castille) n'ont pas encore de jeu d'ambitions écrit
  (elles retombent sur `default`) : voir `docs/wip/camp-r6.md`.
- Hors lot : départs 1356/1415, conseil de saison.
