# Finalisation (v2) — du prototype complet au jeu fini

Date : 2026-09-23 (session 4). Point de départ : M0-M10 terminés (voir `docs/status.md`).

## 1. Constat (audit de début de session 4)

Tout compile, 183 tests Rust + smoke Godot verts. Mais un joueur trouverait encore un prototype :

- **Campagne peu vivante** : sonde IA 100 tours → aucune guerre de 1346 à 1360, 29 batailles en 25 ans,
  l'Empire thésaurise (368 000 livres), banqueroutes à répétition (Navarre 18, Écosse 11, Gênes 8).
  La guerre de Cent Ans doit être *la* guerre : pression dynastique permanente France-Angleterre.
- **Règles inertes** : effets de bâtiments (garnison, coût de recrutement, ravitaillement, ciblage par
  classe), de technologies (`army_upkeep`, `army_experience`, `recruit_cost`, `movement`, `production`,
  `siege_resistance`, `fortification_level`, `wealth`, `prestige`), de traits (`research_*`,
  `Diplomacy`, `Intrigue`, `Loyalty`) affichés mais sans effet ; surplus de recherche perdu ; alliés
  présents dans la province absents de la bataille ; pas de chaînes d'événements ni de capture.
- **Interface austère** : aucune icône (unités, bâtiments, ressources, technologies), peu d'infobulles,
  menu de départ sans illustration, pas de réglages graphiques, pas de sauvegarde automatique, pas de
  rapport de fin de tour, frontières de provinces crénelées.
- **Batailles** : pas de phase de déploiement, pas de collisions entre régiments amis, IA sans
  changement de formation, sièges sans vrai cheminement, tours décoratives.
- **Contenu** : factions manquantes (Venise, Florence, Grenade, Brabant, Hollande-Hainaut,
  Anjou-Provence, Gueldre), Charles VI absent, 48 événements seulement, pas de tutoriel ni d'encyclopédie.
- **Portraits** : 1/50 ; la clé OpenRouter reste bloquée jusqu'au 1er octobre (limite mensuelle propre
  de 100 $ atteinte hors projet). Aucune dépense possible cette session ; tout reste procédural.

## 2. Définition de « fini »

1. Une partie complète 1337-1453 jouable avec chacune des 3 factions sans blocage ni règle inerte.
2. La guerre France-Angleterre occupe la majorité du siècle, avec trêves ; aucune faction ne thésaurise
   plus de 8 saisons de revenu ; les banqueroutes restent rares (< 1 par faction et par décennie).
3. Chaque élément de jeu a une icône et une infobulle ; chaque écran a un retour clair.
4. Premier contact guidé (tutoriel) et encyclopédie tirée des données.
5. Réglages, sauvegardes automatiques et manuelles, crédits (licences des icônes), export macOS.
6. Tests : chaque lot ajoute ses tests ; smoke Godot étendu ; recette jouée des trois factions.

## 3. Lots

Chaque lot a un propriétaire de fichiers exclusif pour que les agents parallèles ne se gênent pas
(voir ADR 0002). Les modifications de `campaign_map.gd` sont limitées à des points d'accroche courts.

### Vague 1

| Lot | Contenu | Fichiers possédés |
|---|---|---|
| **F1 Règles inertes** | Brancher tous les effets inertes listés § 1 (bâtiments, techs, traits), ciblage par classe, surplus de recherche conservé, armées alliées de la province qui rejoignent la bataille (auto-résolution et `battle_setup`), effet d'événement « capturer un personnage » et effets différés (chaînes N+k), Charles VI en naissance historique (1368, fils de Charles V et Jeanne de Bourbon si mariés). Tests. | `core/crates/sim-campaign`, `core/crates/data-model`, `data/schemas`, `data/events`, `data/characters` |
| **F2 Icônes et infobulles** | Icônes SVG de game-icons.net (CC BY 3.0, attribution) téléchargées et normalisées par `tools` (teinte encre sépia), bibliothèque `IconLibrary` (autoload) avec correspondance par identifiant dans `data/` (champ `icon` optionnel ou table), intégration : barre du haut, panneaux province (bâtiments, recrutement, ressources, classes), armée (unités), bataille (cartes d'unités), technologies, cour ; infobulles riches (coût, entretien, effets, prérequis). Fichier `CREDITS.md` + écran de crédits alimenté par lui. | `tools/cent_ans_tools/icons*`, `game/assets/icons/`, `game/scripts/ui/icon_library.gd`, `map_ui.gd`, `province_panel.gd`, `army_panel.gd`, `battle_hud.gd`, `tech_tree_view.gd`, `court_panel.gd` |
| **F3 Écrans et flux** | Menu de départ illustré (fond carte ancienne rendu depuis la heightmap et les côtes, écus des factions, animation légère), écran de chargement avec progression, menu pause (Échap) : reprendre / sauvegarder / charger / réglages / aide / quitter, réglages (plein écran, résolution, vsync, échelle d'interface, pan par bords, vitesse de caméra, sauvegarde auto), sauvegarde automatique (tous les N tours, 3 emplacements tournants) et emplacements nommés avec date de jeu, rapport de saison en fin de tour (événements majeurs cliquables → caméra), alertes (armée ennemie à la frontière, siège, construction/recherche terminée, dette), écran de crédits. | `start_menu.gd/.tscn`, `save_load_dialog.gd`, nouveaux `game/scripts/ui/{settings,pause_menu,season_report,alerts,loading,credits}*`, `project.godot` (entrées) |

### Vague 2

| Lot | Contenu | Fichiers possédés |
|---|---|---|
| **F4 Guerre de Cent Ans vivante** | IA : pression dynastique Plantagenêt (prétention à la couronne de France), guerres qui reprennent après trêves, grandes chevauchées, alliés (Écosse-France « Auld Alliance », Flandre-Angleterre, Bourgogne opportuniste), dépense des trésors dormants (Empire), prévention des banqueroutes, mariages par l'IA, cibles chiffrées § 2.2, sonde multi-graines. | `core/crates/ai`, `diplomacy.rs` (partie IA), `economy.rs` (équilibrage) |
| **F5 Batailles, finition** | Phase de déploiement (zone du camp, glisser les régiments), collisions entre régiments amis, IA qui adopte schiltron/formations, cheminement de siège (grille A*), tir depuis les tours, sortie de garnison, maisons obstacles, renforts alliés en cours de bataille, minicarte de bataille. | `core/crates/sim-battle`, `game/scripts/battle/*` sauf `battle_hud.gd` (F2, à reprendre après fusion) |
| **F6 Rendu de la carte** (transféré à la session parallèle « visual », refonte semi-réaliste) | Frontières lissées (champ de distance ou filtrage bilinéaire du masque), épaisseur selon zoom, étiquettes sans chevauchement et courbées selon la province, marqueurs d'armée lisibles (bannière + effectif), légendes des modes de carte, minicarte cliquable, brouillard de guerre léger (provinces non adjacentes grisées pour les armées). | `game/scripts/map/*` (hors contrôleurs de F3), `game/shaders/*` |

### Vague 3

| Lot | Contenu |
|---|---|
| **F7 Contenu** | Factions manquantes (Venise, Florence, Grenade, Brabant, Hollande-Hainaut, Anjou-Provence, Gueldre) avec provinces, dirigeants réels de 1337, noms ; 30 événements de plus (cible ≈ 80) ; listes de prénoms (Portugal, etc.). |
| **F8 Tutoriel et encyclopédie** | Tutoriel des premiers tours (étapes contextuelles, désactivable), encyclopédie (unités, bâtiments, techs, traits, factions) générée depuis les données, `docs/manuel.md` joueur. |
| **F9 Recette** | Parties complètes automatisées pour les 3 factions, parcours de tous les écrans en captures, corrections, performance de fin de tour, export `.app`, docs finales. |

## 4. Règles de travail (rappel)

Agents `cent-ans-dev` (Opus, effort medium) en worktree, ≤ 3 en parallèle, commits `wip:` ≤ 15 min,
`docs/wip/<lot>.md`, rapport final. L'orchestrateur fusionne, relance les tests et met à jour
`docs/status.md` et `docs/wip/finalisation.md`.
