# 0191 — Plancher d'échelle 3D à 0,4 en plein écran HiDPI
Date : 2026-10-08. Statut : acceptée. Modifie le plancher de l'ADR 0123 (0,5 → 0,4).

## Contexte
Chantier FL (fluidité de la carte). Le joueur joue en plein écran sur un Dell 27" HiDPI 2×
(2048 × 1152 points, 4096 × 2304 pixels). Avec le budget de l'ADR 0123, le préréglage Haute
(0,75) tombe au plancher 0,5 : 2,36 M pixels rendus, carte à 38-43 ms par image (p50), contre
16-20 ms dans la fenêtre par défaut (1440 × 900, 0,73 M px). Mesures A/B en processus
(`--bench-ab`, fenêtre en arrière-plan) : 0,42 → 30 ms, 0,35 → 25 ms. Le coût est d'abord le
shader du terrain, payé au pixel ; aucun effet isolé ne dépasse 6 ms (`docs/wip/fl-fluidite-carte.md`).

## Décision
`RenderQuality.UPSCALE_MIN_SCALE` passe de 0,5 à 0,4 (choix du joueur entre 0,5, 0,4 et 0,35).
Seul le choix « Automatique » est concerné, et seulement au-delà de ≈ 7,3 M pixels de fenêtre
(4K, plein écran HiDPI 27") : l'écran Retina du portable (2624 × 1644) reste à 0,52.

## Conséquences
- Plein écran sur le Dell : 1638 × 922 pixels rendus, ≈ 30 ms au lieu de 40 (−25 %).
- La 3D est un peu plus douce (0,8 pixel rendu par point logique) ; l'interface et le texte
  restent nets (rendus en natif). Les choix explicites (Désactivée, qualité, performance) et
  Ultra ne changent pas.
