# Session 5 — crédit OpenRouter (19 €) et écarts restants

Démarrée le 2026-09-24. Nouvelle clé OpenRouter (≈ 19,6 $ de crédit), tout peut être consommé
(plafond projet 50 $ inchangé, dépenses dans `docs/budget.md`).

## État
- [fait] Portraits (88/88, 3,85 $) : `cent-ans assets portraits` (87 éligibles avec 29 factions, ≈ 4 $).
- [en cours] Miniatures d'événements : nouvel outil `cent-ans assets event-art` (117 × 768×432 JPEG,
  ≈ 5,5 $), bandeau dans la fenêtre de chronique (fait, capture `docs/img/chronicle-miniature.png`).
- [fait] Illustrations de l'encyclopédie (116/116, 5,27 $) : `cent-ans assets illustrations` (unités, bâtiments,
  technologies, factions ; 116 × 640×360 JPEG, ≈ 5,3 $), en tête de fiche (fait, capture
  `docs/img/encyclopedia-illustration.png`).
- [fait] Agent A : bonus des technologies dans la bataille 3D (limite G1), fusionné (abdd929).
- [fait] Agent B : Bourgogne-Angleterre (23/40 graines) et Brabant (39 %), fusionné (dbdcc69) ; 38/40 survies.
- [en cours] Agent C (worktree) : G5, `are_neighbors` sur l'adjacence de carte + rééquilibrage (survie 40/40).
- [à faire] Après génération : `godot --headless --path game --import`, commiter images + `.import`,
  vérifier `docs/budget.md` contre `GET /api/v1/credits` (total_usage de départ : 80,3766 $).

## Reprise
- Les deux générations sont idempotentes : relancer la commande reprend les images manquantes.
- Sonde non consignée automatiquement : 1 appel `image_config` 16:9 (0,04 $, ignoré par le modèle).
