# Lot G4 « Bourgogne et Brabant » — état

Branche : `worktree-agent-a734b058e294e9537` (à partir de `main` ac6b7c2). Périmètre : `core/crates/ai`
(alignement), `data-model` (fichier de réglages), `sim-campaign` (deux constantes exposées), réglages dans
`data/ai/alignment.json`. Écarts visés (`docs/status.md`, tableau G2) : alliance anglo-bourguignonne jamais
conclue ; Brabant rarement anglais.

## Mesure de départ (`century_probe`, graines 1-5, 464 tours, main ac6b7c2)
- Bourg.-Angl. : 0 % partout (graines 1-10) ; Angleterre dominante 0/0/0/3/0 % (max 3-10 provinces).
- Angl.-Brabant : 0 % partout (graines 1-5) ; 29/0/0/0/0 % (graines 6-10).
- Guerre FR-EN 53/75/58/55/25 % (moy. 53 %) ; survie 1400 : 4/4 partout (graines 1-10).
- Base de comparaison : worktree détaché `scratchpad/base` (ac6b7c2), graines 6-10 dans `base_6_10.txt`.

## Diagnostic
- `TRACE=1 century_probe 464 <graine>` : état de la Bourgogne, du Brabant et du Hainaut tous les 5 ans,
  événements citant la Bourgogne, grief calculé.
- Montereau (1419) et « Sceller l'alliance avec Henri V » (1420) se déclenchent souvent, mais l'attitude
  globale envers la France reste ≥ 0 (même maison Valois, loyauté, présents) : un seuil d'attitude ne voit
  jamais le grief. D'où un grief mesuré sur les **modificateurs d'opinion négatifs en cours**.
- **Bogue trouvé** : `CampaignState::are_neighbors` lit `Province::neighbors` des fichiers de province,
  renseigné pour 6 provinces sur 132 : presque aucune faction n'est « voisine ». Le corriger globalement
  (essai : géométrie `movement::land_neighbors`) quintuple les appels aux armes (≈ 150 → 800-950), fait
  monter la guerre FR-EN (66-86 %) et casse l'Auld Alliance sur 2 graines : hors périmètre, non retenu.
  L'IA d'alignement utilise sa propre fonction `alignment::borders` (géométrie).

## Points
1. [x] Grief (`grievance_change`) : somme des modificateurs négatifs ≤ `grievance.grudge` envers son
   suzerain/allié/ennemi, bascule vers le prétendant ou l'ennemi de celui-ci (attitude hors guerre
   ≥ `min_attitude`). Pas de jet : la chronique a déjà hésité (choix pondérés de Montereau et des Armagnacs).
2. [x] Domination relative (`dominance_realm_share` = 15 % du royaume au lieu de 8 provinces).
3. [x] Réglages dans `data/ai/alignment.json` (struct `AiAlignment` dans data-model). [ ] schéma + test Python.
4. [x] Fiefs-rentes (`plan_money_fief`) + princes courtisés seulement dans une guerre de succession, pas
   pour un vassal, jamais un ennemi/prétendant de nos alliés.
5. [ ] Tests Rust G4, mesures finales, docs/status.md, docs/design/m9-ai.md § 5.

## Mesures intermédiaires (run6, graines 1-5 puis 6-10)
- FR-EN 57/70/50/71/37 (moy. 57 %) ; 60/64/62/64/49 (moy. 60 %).
- Bourg.-Angl. : 3/5 graines (1419-1431) ; 4/5 graines (1418-1421).
- Angl.-Brabant (tours de guerre FR-EN) : 0/0/19/32/50 (moy. 20 %) ; 83/0/88/0/2 (moy. 35 %).
- Écarts à surveiller : Écosse détruite avant 1400 (graine 3), Navarre 32 banqueroutes/déc. (graine 10).

## Prochaine étape
Examiner Écosse graine 3 et Navarre graine 10 ; schéma JSON + test Python ; tests Rust ; docs.
