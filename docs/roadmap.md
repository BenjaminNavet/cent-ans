# Feuille de route

Mise à jour : 2026-10-22. Les jalons M0 à V6 sont ceux de la v1 (septembre 2026) ; les jalons
après V6 ont été ajoutés le 22/10 à partir des chantiers réels (`docs/wip/`, ADR). État détaillé :
`docs/status.md`.

## Jalons v1 (2026-09)

| Jalon | Contenu | État |
|---|---|---|
| M0 | Fondations : outils, dépôt, squelette Rust+Godot, schémas, docs | terminé (2026-09-23) |
| M1 | Carte de campagne réelle (provinces, terrain 3D, caméra, villes) | terminé (2026-09-23) |
| M2 | Boucle de campagne minimale jouable (tours, armées, auto-résolution, sauvegarde, UI) | terminé (2026-09-23) |
| M3 | Villes et économie (classes, bâtiments, jauges, ressources) | terminé (2026-09-23) |
| M4 | Personnages et dynasties | terminé (2026-09-23) |
| M5 | Diplomatie et religion | terminé (2026-09-23) |
| M6 | Technologies | terminé (2026-09-23) |
| M7 | Batailles temps réel 3D | terminé (2026-09-23) |
| M8 | Sièges | terminé (2026-09-23) |
| M9 | IA de campagne et de bataille | terminé (2026-09-23) |
| M10 | Assets, sons, événements, équilibrage | terminé (2026-09-23) ; 49 portraits sur 50 à générer (limite de la clé OpenRouter) |
| F1-F9 | Finalisation : règles inertes, icônes et infobulles, écrans et flux, guerre vivante, batailles (déploiement, sièges, renforts, HUD), factions et événements, tutoriel et encyclopédie, recette | terminé (2026-09-24), voir `docs/design/v2-finalisation.md` |
| G1-G2 | Dernières règles (recrutement, piété, cessions, rançons, alliés aux assauts) et IA historique (subsides, Pays-Bas, trésors) | terminé (2026-09-24) |
| V1-V6 | Refonte visuelle semi-réaliste (session parallèle « visual », ADR 0004) | terminé (2026-09-24) |

Note : les mentions « Godot » des jalons M0 à M10 décrivent l'époque du client Godot, retiré depuis
(ADR 0366). Le client actuel est en Bevy (ADR 0357).

## Jalons après V6 (2026-09-26 → 2026-10-22)

| Jalon | Contenu | État |
|---|---|---|
| Portage Windows | Lanceur `Lancer Cent Ans.exe` depuis les sources (ADR 0087, 0153) | code terminé (2026-09-26 et 2026-10-02) ; essai sur un vrai PC restant (joueur) |
| Seed bots | Studio de test autonome sur le cœur (ADR 0348) ; correctifs et faillites (ADR 0350) | terminé (2026-10-13) ; ligne de base non figée |
| Migration Bevy | Client Bevy 0.20 (ADR 0357), lanceurs Bevy (ADR 0362), retrait de Godot (ADR 0366) | fusionné dans `main` (2026-10-17), **non poussé** |
| Playtest 2026-10-17 | 7 testeurs, 11 lots de correctifs | terminé (2026-10-17) ; cible 40 i/s en ultra non atteinte |
| Qualité visuelle (VQ) | Lots de qualité de rendu | 8 lots fusionnés (2026-10-17) ; mesure de la cible 40 i/s à refaire |
| Style Total War (TS) | Neige, forêts, brume, villes, armées, plaques, sous-bois, bosquets | lots lancés le 2026-10-20 ; état de chaque lot dans `docs/wip/ts-style-total-war.md` |
| Choc des charges | Modèle en données (ADR 0711) | fusionné (2026-10-20) ; rééquilibrage de l'IA à faire |
| Refonte des systèmes | Révoltes, mêlée, économie et IA de campagne (`docs/wip/refonte-systemes.md`) | révoltes : mesure faite (lot A, 2026-10-22), règles à faire ; mêlée, économie et IA non commencées |
| Backlog BL | Traitement du backlog par agents (`bl/*`) | **en pause** depuis le 2026-10-17, hors `main` ; tri ou abandon à décider |
| Meshy MX | Espèces d'arbres, habitats, bosquets (vagues 11-13) | dans `main` ; publication des modèles bloquée |
| Nuit de playtest 2026-10-22 | Test de nuit, mandat large sans protocole écrit | note créée, sans constats (`docs/wip/nuit-playtest-2026-10-22.md`) |

## Décisions en attente (joueur)

- **Définir la version jouable** : tranche de contenu, critères de sortie (durée de partie, cible de
  fps sur machine de référence, absence de bloquants, tutoriel terminé sans aide). Aucun critère n'est
  écrit à ce jour.
- **Plafond budgétaire** : 136,39 $ dépensés contre 50 $ de plafond v1 (voir `docs/status.md`).
- **Push** : 996 commits non poussés vers `origin/main` ; sauvegarde distante à choisir.
- **Branches `bl/*`** : trier ou abandonner avant de lancer la refonte économie / IA.
- **Publication des assets** : inventaire des licences avant toute diffusion publique du dépôt.
