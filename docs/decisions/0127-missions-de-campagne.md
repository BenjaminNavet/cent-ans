# ADR 0127 — Missions de campagne à court terme

Date : 2026-09-29. Statut : accepté. Chantier NT, lot NT3
(`docs/superpowers/specs/2026-09-29-nt-nuit-tww3-design.md`, note `docs/wip/nt3-missions.md`).

## Contexte

Le joueur n'avait que des objectifs historiques de long terme (`victory.rs`,
`feudal/objectives.rs`). TWW3 rythme la campagne par des missions courtes (quelques tours) à
récompense immédiate. Il fallait les ajouter sans toucher à l'équilibre de l'IA ni à la suite
aléatoire de la campagne (tests d'équilibre graine-dépendants : `ep7_historical`, EQ6...).

## Décision

- Module `sim-campaign/src/missions.rs`, gabarits dans `data/missions.json`
  (schéma `data/schemas/missions.schema.json`) : six genres — prendre une province voisine tenue
  par un ennemi en guerre, gagner n batailles, achever un bâtiment disponible dans une place
  donnée, recruter/engager n unités, conclure un traité (paix, alliance, vassalité), tenir une
  place frontalière menacée jusqu'à l'échéance.
- **Joueur seulement** : l'IA n'en reçoit pas. 1 à 2 missions actives (`max_active`), jamais deux
  du même genre ; au plus une proposition par tour, après `first_turn` et un délai
  (`offer_cooldown_turns`) après chaque réussite ou échec.
- **Selon la situation** : un gabarit n'est tiré (au poids) que s'il a une cible plausible
  (voisin en guerre, bâtiment `available` dont la durée tient dans l'échéance, unité recrutable,
  faction non alliée...). La cible est tirée parmi les plausibles.
- **Déterminisme** : générateur propre, graine = graine de campagne × tour ; on ne tire jamais
  dans `CampaignState::rng`, dont la suite reste celle d'avant NT3.
- **Résolution en fin de tour**, en dernier (après la victoire, la nouvelle saison commencée) :
  réussite → récompense (or au trésor, prestige du souverain, agitation retirée à la province
  visée sinon à la capitale, compagnie gratuite — la plus chère levée dans la capitale — dans sa
  garnison) ; échéance passée ou place perdue → échec, `failure_prestige` (−2, borné à −20..0).
- **Suivi** : batailles et recrutements par compteurs (crochets dans `battle_outcome::apply` et
  `submit_order_outcome`, donc ordres du joueur seulement) ; prise, bâtiment, place tenue et
  traités par l'état (traités : comparaison à un instantané alliances/trêves/suzerain/vassaux).
- **Sauvegarde** : champ `CampaignState::missions` en `serde(default)`, sans changer
  `STATE_VERSION` ; les avis du dernier tour (`notices`) sont sauvegardés aussi (un état
  rechargé est égal à l'état sauvé).
- **Interface** : pont `get_missions` / `get_mission_notices` ; section « Missions » du panneau
  d'objectifs (touche O), avis en toast à l'obtention, la réussite et l'échec ; genre d'événement
  `mission` dans le journal et le rapport de saison.

## Conséquences

- Les récompenses ajoutent un peu d'or et de prestige au joueur seul : à surveiller par la partie
  pilote (ordre de grandeur : quelques centaines de livres toutes les 4 à 10 saisons).
- Les batailles gagnées en défense pendant le tour de l'IA comptent (auto-résolues contre le
  joueur) ; les assauts de siège ne passent pas par `battle_outcome::apply` et ne comptent pas.
- Ajouter un genre de mission = une variante de `MissionKind`, ses candidats et son verdict dans
  `missions.rs`, et l'énumération du schéma.
