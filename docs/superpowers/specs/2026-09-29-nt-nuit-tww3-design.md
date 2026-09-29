# NT — nuit « niveau TWW3 » (29-30/09)

Date : 2026-09-29. Statut : spec auto-validée (mandat du joueur : « autonomie totale toute la nuit
pour rapprocher le jeu de Total War: Warhammer 3 »). Coût cloud prévu : 0 $.

## Cadre

Audit des écarts restants avec TWW3 (29/09, vérifié dans le code) : l'essentiel des audits A1-A3
est fait. Restent ouverts, hors zones prises par d'autres sessions (DV deux vues, FK carte vivante,
FPS carte — toute la carte de campagne est donc exclue) :

- **Faisables cette nuit sans assets payants** : variété des sièges, bataille personnalisée,
  missions de campagne, bataille-prologue guidée, plafond d'unités (N6), engins construits sur
  place (N7), petites suites TW2/Q6.
- **Hors nuit, décision du joueur requise** (assets externes) : animation par capture de
  mouvement et combats appariés cuits (banque mocap sous licence commerciale), textures peintes des
  figurines et nouveaux types d'unités (GA3 bloqué sans clé fal.ai), partition musicale composée.
  Voir § « Pour le joueur ».

## Lots

| Lot | Contenu | Zone | Agent |
|---|---|---|---|
| NT1 Sièges variés | Types de places : **château** (enceinte polygonale serrée + donjon + basse-cour, peu de maisons), **bourg fortifié** (enceinte + rue principale + place de marché, maisons en bandes le long des rues), **cité** (plan en anneaux actuel). Type choisi par une règle de données (rang, fortification, tag « château » éventuel) dans `data/rules/siege_town.json` ; les 7 plans emblématiques inchangés. Variations déterministes par graine de province (orientation, nombre de côtés, position du donjon/porte). Rendu : réutilise les maquettes existantes (murs, tours, maisons, donjon si présent, sinon tour agrandie). | `core/crates/sim-battle/src/siege.rs` (+ module `siege_layouts.rs`), données, rendu siège | cent-ans-dev |
| NT2 Bataille personnalisée | Entrée « Bataille personnalisée » du menu principal : deux camps (faction parmi les jouables), budget de points (défaut 6000, réglable), achat d'unités depuis le roster de la faction (coût = coût de recrutement), champ (type de terrain, saison, météo, heure), option siège (place de l'un des types NT1). Lance la bataille par le chemin existant des batailles historiques/démonstration. Composition mémorisée entre deux lancements. | `start_menu.gd`, nouvel écran `custom_battle_screen.gd`, pont éventuel | cent-ans-dev |
| NT3 Missions de campagne | Module `sim-campaign/src/missions.rs` : missions à court terme (3-12 tours) tirées de `data/missions.json` (prendre une province voisine, gagner une bataille, construire un bâtiment, recruter N unités, conclure un traité, tenir une place), récompense (or, prestige, ordre public, unité), 1 à 2 actives par faction joueur, générées selon la situation, échec à l'échéance. UI : section « Missions » du panneau d'objectifs + avis à l'obtention/réussite/échec. L'IA n'en reçoit pas. | core + UI objectifs | cent-ans-dev |
| NT4 Bataille-prologue | Bataille guidée courte (ex. escarmouche de 1337 en Guyenne) depuis le menu (« Didacticiel de bataille ») : étapes scriptées (caméra, sélection, déplacement, formation glisser, charge, tir, pause, victoire), infrastructure du tutoriel de campagne + conseiller. Données dans `data/tutorial/battle_prologue.json`. | UI bataille, données | cent-ans-dev (vague 2) |
| NT5 N6 + N7 | N6 : plafond d'unités par armée au recrutement (20, donnée), erreur explicite dans l'UI de recrutement, IA respectant le plafond. N7 : pendant un siège de campagne, construction d'engins (béliers, échelles, beffroi) sur N tours selon la taille de l'armée ; assaut possible avant la famine ; l'IA construit puis donne l'assaut. Garde-fous : `ep7_historical`, `ep9b_duel`, `ai_beats_a_passive_ai_at_equal_forces`, test EQ6 (part de guerre FR–EN). | `sim-campaign` (recrutement, siège), UI recrutement/siège | cent-ans-dev |
| NT6 Petites suites | a) discours du général ennemi (voix existante ou texte), indicateur « visé » sur béliers/beffrois ; b) surprime des mercenaires dans le panneau budget, en-tête du panneau de province compacté (≤ 5 lignes en 1280×720), avis longs contenus ; c) IA au repos pour se reconstituer (posture existante) ; d) réaffectation libre des touches dans Réglages (liste des actions InputMap, capture de touche, sauvegarde dans les réglages). | divers | cent-ans-mech (vague 2) |

Hors périmètre : carte de campagne (marqueur de ruine, traditions sur le marqueur 3D → DV/FK),
guerre civile/prétendants (L, à spécifier avec le joueur), batailles navales 3D.

## Organisation

- Intégration : worktree `../game_project-nt`, branche `integration/nt` ; chaque lot sur
  `feat/nt<N>-…` en worktree d'agent ; fusion dans `integration/nt`, vérifs complètes, puis
  `merge --ff-only` dans main (rebase/merge de main d'abord : d'autres sessions commitent).
- Vague 1 : NT1, NT2, NT3, NT5 (4 agents ; FK a encore un agent). Vague 2 : NT4, NT6a-d.
- ADR : numéros 0126-0129 réservés pour NT (0124-0125 laissés à DV/FK). NT1 (types de places)
  et NT3 (missions) en ont chacun un.
- Vérifs par lot : `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test` (target privé), pytest
  si données, `smoke.gd`, test Godot du lot. Captures : 3 au plus pour toute la nuit côté session
  principale (sièges NT1, écran NT2) ; les agents n'en font pas.

## Critères de réussite

- Un siège d'une place non emblématique montre l'un des trois types, différent selon la place.
- On peut lancer une bataille personnalisée composée à la main depuis le menu.
- Une partie donne des missions qui se résolvent (réussite/échec) avec récompense visible.
- Le recrutement refuse la 21e unité ; un siège de campagne peut finir par un assaut après
  construction d'engins, sans casser l'équilibre EQ6.
- Tous les tests verts dans main au matin, note `docs/wip/nt.md` à jour.

## Pour le joueur (à trancher au réveil)

1. Mocap : acheter une banque de capture de mouvement sous licence commerciale (ordre de grandeur
   50-300 $, hors enveloppe v1) pour le corps à corps et les combats appariés ?
2. Clé fal.ai pour GA3 (image → 3D) afin d'enrichir figurines et roster ?
3. Guerre civile / prétendants (Armagnacs-Bourguignons) : chantier suivant ?
