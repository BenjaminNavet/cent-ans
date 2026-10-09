# État de l'application

Dernière mise à jour : 2026-10-09 (synthèse rédigée le 2026-09-24 ; historique dans `docs/archive/status-historique.md`).

## Où en est-on
- **Jeu complet et finalisé** : campagne 1337-1453 (1477 pour la Bourgogne) jouable de bout en bout avec les
  trois factions, 29 factions au total, 117 événements de chronique dont des chaînes, IA qui mène une vraie
  guerre de Cent Ans (France-Angleterre en guerre 54-76 % du siècle, 3 à 8 phases, alliances historiques),
  batailles et sièges 3D avec phase de déploiement, rendu semi-réaliste, icônes et infobulles partout, menu
  illustré, réglages, sauvegardes automatiques, rapport de saison, alertes, tutoriel, encyclopédie (L), codex
  historique (K), manuel (`docs/manuel.md`), crédits, export macOS vérifié (`tools/export_macos.sh`).
- **Victoire** : tous les objectifs historiques tenus `hold_turns` saisons d'affilée (France 20, Angleterre et
  Bourgogne 12). Sonde : `cargo run --release -p ai --example playthrough [graine]`.
- **Qualité** : 310 tests Rust, 87 tests Python, smoke Godot 20 étapes, clippy propre.
- **Art** : 88/88 portraits, 117/117 miniatures de chronique (bandeau de la fenêtre de chronique,
  `cent-ans assets event-art`), 116 illustrations de l'encyclopédie (unités, bâtiments, technologies,
  factions, `cent-ans assets illustrations`), miniature en tête des 231 fiches du Codex (84 générées par
  `cent-ans assets codex-art`, les autres reprennent l'image de leur `entity` ; capture
  `docs/img/codex-window.png`) ; 19,20 $ dépensés sur 50 $ (`docs/budget.md`).
- **Reste** : écarts d'équilibrage documentés plus bas (tableaux F4, G2, G4, G5) ; G5 : voisinage réel
  (graphe de la carte), 4 grandes factions en vie en 1400 sur 40/40 graines.
- Plan et suivi de la finalisation : `docs/design/v2-finalisation.md`, `docs/archive/chantiers.md`.
- Design validé : `docs/design/2026-09-23-cent-ans-design.md`.

## Ce qui fonctionne
Journal détaillé jalon par jalon (M1-M10, F1-F9, G1-G5, sessions suivantes) : `docs/archive/status-historique.md`. Vue d'ensemble du code : `docs/architecture.md`. Décisions : `docs/decisions/INDEX.md`. Chantiers récents : `docs/wip/`.

## Limites connues
- F2 : `GameDataStore` n'expose pas les définitions d'unités, bâtiments, ressources et technologies ;
  les infobulles lisent ces JSON de `data/` via `GameCatalog` (affichage seul ; coûts effectifs,
  disponibilités et refus viennent de `CampaignSim`). À remplacer par des accesseurs Rust
  (`get_unit_type`, `get_building`, `get_resource`). Forces/faiblesses d'unité : statistiques à
  ±30 % de la moyenne des types d'unités (heuristique d'affichage). Les infobulles des traits
  n'ont qu'une icône par catégorie, celles des compétences une par branche.
- M10 assets : les images générées sont des JPEG/PNG chargés à la volée (`PortraitLoader.load_texture`) ;
  tout le jeu fonctionne sans elles (écu de faction ou bandeau masqué en repli).
- M10 assets : headless, `AudioDirector` charge les flux sans les jouer (le pilote factice fuit les lectures OGG).
- `get_faction_summary` renvoie 0 pour projected_income/upkeep avant le premier tour (champs mis en cache en fin de tour) ; l'interface utilise `get_faction_economy` qui calcule à la volée.
- Équilibrage (M10) : frais de cour et d'administration = 8 % du revenu + 1 % par province (plafond 35 %) + 3 % du trésor au-delà de huit saisons de revenu (F4 : 20 % au-delà de six saisons) ; débarquement en terre hostile (mouvement épuisé, −5 % d'hommes, −10 l'hiver, −10 de moral) ; l'IA n'envahit par mer que les provinces qu'elle revendique, rembourse ses dettes en 20 tours (licenciements groupés, tribut compris) ; les guerres contre une faction disparue prennent fin. Sur 5 graines × 464 tours : l'Angleterre survit partout (revenu ×2 à ×3), la France domine, l'Empire garde un trésor élevé (≈ 15 saisons de revenu) : à surveiller.
- L'IA minimale recrute une unité par tour et thésaurise ; l'IA complète est M9.
- La population ne varie pas encore (M3).
- Factions manquantes (Anjou-Provence, Grenade, Hollande-Hainaut, Brabant, Gueldre, Venise, Florence…) remplacées par la faction la plus proche, voir `docs/design/provinces-1337.md`.
- L'addon Blender MCP exige Blender ouvert en mode graphique ; le fallback headless est `tools/cent_ans_tools/blender.py`.

- L'IA ne propose pas encore de mariages (M9).
- M5 : l'IA diplomatique est volontairement prudente (peu de déclarations de guerre) ; la guerre de Cent Ans peut se conclure tôt par une paix blanche. Les noms des maisons générées viennent de la capitale ; le Portugal n'a pas de liste de prénoms dédiée.
- M6 : le mock GDScript n'a pas de technologies.
- F1 : l'interface n'affiche pas encore la ventilation par classe des effets (exposée par le pont :
  `effects.by_class`).
- G1 : la bataille 3D applique désormais les bonus des technologies de chaque camp (moral, mêlée, tir,
  armure, par catégorie d'unité) en plus de ceux des bâtiments de la province de levée dans `battle_setup`
  (`docs/archive/chantiers.md`) ; l'IA ne libère jamais un captif contre rançon d'elle-même
  (elle paie, ou libère sur parole les simples chevaliers). Pas de faction Danemark : l'achat du Jutland
  (`evt_valdemar_iv`) reste sans cession. Le plafond de recrutement (G1) limite l'IA à 3 levées par tour
  dans sa capitale et 2 ailleurs (hors bâtiments) : trésors à resurveiller avec `century_probe`.
- Les mariages ne sont pas inscrits au journal de la simulation (ordres immédiats) : l'interface affiche un message ; l'IA ne marie encore personne (M5).
- Batailles (M7) : pas de collisions entre régiments amis ; ligne de vue simplifiée (relief, forêts) ; les engins de siège tirent comme des archers lourds en bataille rangée.
- Sièges 3D (M8) : pas de vrai cheminement (les ordres contournent une seule ouverture à la fois ; un
  ordre à travers la ville entière peut longer un mur) ; les murs bloquent par le centre des régiments,
  leurs rectangles peuvent déborder sur la maçonnerie ; tours de la muraille décoratives (pas de tir
  depuis les tours) ; bélier implicite pour tout assiégeant ; pas de sortie de la garnison pendant la
  bataille ; maisons décoratives (pas d'obstacle). Les assauts d'IA sont courts (3-6 min) face à une
  petite garnison.
- IA de bataille (M9) : pas de manœuvre d'encerclement coordonnée ni d'usage du relief en attaque ;
  l'IA ne change pas de formation (schiltron) d'elle-même.

- M10 : les événements globaux (Peste noire, Constance) attendent le choix du joueur : la vague de peste
  ne démarre qu'une fois la décision prise (ou expirée). La folie vise Charles VI : si la chronologie
  diverge (Charles V non marié à Jeanne de Bourbon, Charles VI mort ou ne régnant pas en 1392-1394), elle
  n'a pas lieu. Brétigny garde le repli « Poitiers a eu lieu » quand Jean II n'est plus captif.

## Commandes
- Build + tests : voir `CLAUDE.md`.
- Budget : `cd tools && uv run cent-ans budget show` (dépensé : 0,00 $ / 50 $).

> Note RX (ADR 0246) : les sondes `century_probe`, `balance_probe`, `ia_quality_probe`, `ai_duel_probe` ont été supprimées (`c2308153d`) ; la mesure de campagne se fait désormais avec `cargo run --release -p ai --example campaign_probe` (ADR 0246).
