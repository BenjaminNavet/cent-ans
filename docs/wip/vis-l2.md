# VIS-L2 habitat à l'échelle (ADR 0824)

Spec : docs/audits/2026-10-30-juge-visuel-iter3.md, G1-2 et lot L2.

## État
- Squelette : note + ADR réservé.

## Plan
1. Données : bloc `habitat` de `data/map/map_countryside.json` (+ schéma) : plafond d'échelle (× réel), portée max des
   habitations. Props marquées `"habitat": true`.
2. `countryside/{config,layer}.rs` : `prop_frame` applique le plafond et la portée des habitations ; fin du grossissement
   « taille écran » pour les fermes.
3. `towns/shapes.rs` supprimé : plus de bloc maison ni d'arbres de jardin bâtis par le code (kit glb partout, arbres de
   la végétation en place). Test d'inventaire (grep).
4. Noyau de hameau : décalque au sol + au plus une maquette existante.
5. Figures et chariots plafonnés relativement aux maquettes de ville.

## Prochaine étape
Étape 1.
