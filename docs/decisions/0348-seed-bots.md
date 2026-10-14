# 0348 — Seed bots : studio de test autonome sur le cœur

Statut : accepté (13/10).

## Contexte
Les tests unitaires vérifient des cas construits à la main ; aucun ne joue de vraies parties. Des
défauts n'apparaissent qu'après des dizaines de tours (panique, invariant cassé, ordre déclaré
possible puis refusé, sauvegarde qui ne reboucle pas). Le cœur Rust est headless et déterministe : on
peut y faire jouer des bots sans Godot. Ce studio doit rester indépendant du moteur d'affichage (verdict
Bevy P0 encore ouvert, ADR 0347 réservé) et servir de filet de sécurité automatique.
Spec : `docs/superpowers/specs/2026-10-13-seed-bots-design.md`, plan associé, note
`docs/wip/seed-bots.md`.

## Décision
1. **Crate `core/crates/seed-bots`** (binaire `seed-bots`) : 8 bots aux styles différents, jouant des
   `sim_campaign::Order` (pas de type `Action` : tous les ordres du joueur en sont déjà des variantes).
   `sim-campaign/src/invariants.rs` porte les invariants du cœur.
2. **Quatre familles d'oracles** : (1) invariants d'état, (2) paniques et lenteurs, (3) ordres refusés
   et bouclage de sauvegarde, (4) dérives d'équilibre comparées à des bandes.
3. **Refus signalés seulement pour un ordre vérifié** : les candidats légaux sont recalculés avant chaque
   soumission ; un refus n'est un constat que si l'ordre figure parmi eux. Un refus de contrepartie
   diplomatique est une issue de jeu, pas un bogue.
4. **Bandes d'équilibre** dans `data/balance/seed_bots_bands.json` (JSON, comme le reste des données ;
   schéma dans `data/schemas/`).
5. **Profil Cargo `seed-bots`** (optimisé mais rapide à compiler) ; rapports `report.json`
   (déterministe) et `report.md`, commande `replay <constat.json>` pour rejouer un constat au même tour.
6. **Trois niveaux d'exécution**, durées MESURÉES (machine chargée) :
   - chaque commit : tests unitaires des oracles, < 1 s ;
   - fin de lot et CI (`.github/workflows/seed-bots.yml`, séparé de `windows.yml`) :
     `run --quick` = 8 bots × 1 graine × 20 tours, ≈ 20-30 s ;
   - passe longue : `run` = 8 bots × 4 graines × 150 tours, ≈ 12 min, plafond dur 15 min (rapport
     partiel au-delà). Les 10 graines de la spec (≈ 30 min) étaient trop longues ; plus de graines via
     `--seeds` pour les bandes.
7. **Gouvernance** : la passe longue est lancée par l'agent en arrière-plan sans attendre de feu vert (addendum du 13/10) ; ses constats lui
   sont présentés et il choisit ce qui entre dans le backlog `docs/wip/seed-bots-findings.md`.

## Conséquences
- Les bots d'interface (qui joueraient à travers Godot) ne viendront qu'après le verdict Bevy P0.
- Les ordres sans requête de prérequis côté sim ne sont pas générés comme candidats, donc leurs refus
  ne sont pas vérifiés (liste dans `docs/wip/seed-bots.md`) ; la plupart sont couverts par le bot
  `chaos` sans vérification de refus. Ajouter une requête de prérequis les fait entrer dans le filet.
- La CI ne bloque pas `stable` : un constat est une information, pas une barrière pour les joueurs.
- Tests : `cargo test -p seed-bots` (oracles), `cargo test -p seed-bots --release -- --ignored`
  (déterminisme, rejeu).

## Addendum du 13/10/2026
Le joueur lève le feu vert pour la passe longue : elle tourne sur la machine locale, sans coût,
donc l'agent la lance d'office en arrière-plan après une grosse fonctionnalité. Le tri des constats
pour le backlog reste présenté au joueur ; un bogue franc trouvé par les bots peut être corrigé
directement.
