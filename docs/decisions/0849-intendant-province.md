# 0849 — Intendant de province

## Contexte
Au départ, la France a 32 localités sans chantier ; bâtir prend 4 clics par chantier, soit environ 130 clics, et l'alerte
permanente devient du bruit (audit `docs/audits/2026-10-29-experience.md`, C4 ; juge 30/10, M2 : trésor qui dort).

## Décision
- Règle du cœur : chaque province peut avoir un intendant avec une politique (`best_yield`, `fortify`, `none`) qui lance
  des chantiers dans les localités libres, dans la limite d'un budget par saison. Politiques et budgets en données.
- Activé par défaut en mode apprentissage et pour les nouvelles parties ; réglable par province et globalement.
- L'alerte « localités sans chantier » est retirée ; une ligne d'agenda n'apparaît que si le trésor dort (ADR 0749).

## Conséquences
- Le joueur peut toujours bâtir à la main ; l'intendant ne touche pas une localité qui a déjà un chantier.
- Les seed bots mesurent le trésor dormant au départ.
- Plan : `docs/wip/apprentissage.md`, lot INTENDANT.
