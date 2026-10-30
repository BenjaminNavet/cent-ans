# ADR 0812 — Budget d'attention et Audience de saison (XP-AUDIENCE, R1)

Date : 2026-10-30. Statut : accepté (lot XP-AUDIENCE, boucle juge 2). Numéro à attribuer à la fusion.
Révise l'ADR 0752 (bandeau Conseiller unique), 0775 (avis ordonnés), 0789 (conseiller de saison : devient une source), 0304 (offre de
missions : passe en Affaire) et 0281 (rapport de saison, déjà remplacé par le Récit, ADR 0810). Complète l'ADR 0805 / R1-UI (choix des
ambitions) et l'ADR 0810 (le Récit fournit la cause de l'Affaire du trésorier).
Source : `docs/audits/2026-10-29-experience.md` § C1, R1 et « Lot XP-AUDIENCE ».

## Contexte

Neuf canaux parlaient au joueur en même temps (cloche, bandeau Conseiller, carte « Que faire », fil, journal, lettres, toasts, fenêtre
de décision, fenêtre d'offre de mission). Chaque lot avait ajouté son canal ; la priorité était réglée canal par canal, jamais entre
les canaux. Le joueur apprenait à tout ignorer, y compris l'offre de mission qui expirait et la décision bloquante.

## Décision

1. **Un budget d'attention dans le cœur.** `game_api::agenda::agenda(data, state, faction, events)` range toutes les sources en
   **Affaires** (au plus `max_affairs` = 3, seules poussées au joueur), **À surveiller** (au plus 5) et **Nouvelles** (comptées par
   rubrique, repliées). Sources : alertes de la cloche (`turn_readiness`), offre de mission, choix des ambitions de l'acte
   (`act_view.ambition_pending`), solde net négatif du trésor (avec la `cause_id` du Récit, bouton « Pourquoi ? »). Le tri et les
   plafonds sont des données : `data/rules/agenda.json` (schéma `agenda_rules.schema.json`, `data_model::AgendaRules`), ordre
   croissant, sources `guaranteed` toujours Affaires (décision bloquante, offre de mission), sources `grouped` repliées en une ligne
   avec un compte (armées ennemies, armées sans ordre, incidents de carte). Ce qui ne tient pas dans les trois places passe en « À
   surveiller », puis en Nouvelles. Chaque Affaire porte son `speaker` (titulaire de la charge du conseil, sinon le chroniqueur ou le
   conseil vacant), son `action` (cible d'interface) et son `cause_id`. Les répliques sont en données : `data/ui/audience.fr.json`
   (schéma `audience_texts.schema.json`) ; le client n'écrit aucun texte de l'Audience.
2. **Une décision forte à la fois.** `sim_campaign::chronicle::fire` reporte d'une saison (évènement reprogrammé via
   `ScheduledEvent`, il n'expire donc pas pendant le report) toute décision bloquante hors crise quand le joueur en détient déjà
   `max_strong_decisions_per_season`. Les crises (`crisis_events` : peste noire, révoltes) s'ouvrent aussitôt.
3. **Invariants (seed bots).** `mission_offer_unseen` (une offre vit au moins une saison après sa création : elle est Affaire garantie
   pendant ce temps) et `strong_decisions_open` (jamais plus de décisions fortes ouvertes que le plafond).
4. **Choix des ambitions porté par l'Audience** (points ouverts de R1-UI) : au tour 1 et à chaque nouvel acte, `ambition_choice` est une
   Affaire (« choisissez deux ambitions ») dont le bouton ouvre le panneau d'objectifs ; `ActAmbitionRow.locked` grise les
   ambitions non prises quand le choix est figé.
5. **Client.** `hud/audience.rs` : carte d'Audience (portraits des conseillers, un bouton par Affaire) poussée une fois par saison
   et à chaque Affaire nouvelle, repliable en une ligne, qui devient le Registre (À surveiller + Nouvelles résumées) ; le compteur de
   la cloche ouvre le Registre ; le sceau de fin de saison dit quelle Affaire bloque. Retirés : la fenêtre de décision qui s'ouvrait
   seule, la fenêtre d'offre de mission, la carte « Que faire » (le bandeau Conseiller ne garde que les répliques vocales), les
   toasts d'information des missions. Les noms `hud:*` des scripts de fumée sont conservés. Sonde `hud/attention.rs` (dump
   `attention`) : compte ce qui s'ouvre seul, par saison.

## Conséquences

- Mesure (smoke `campaign_attention`, Angleterre, 10 saisons) : ouvertures d'office 61 -> 12 (33 toasts, 9 bandeaux, 9 fenêtres de décision, 10 fenêtres d'offre avant ; 11 cartes d'Audience et 1 bandeau après). Détail dans `docs/wip/xp-audience.md`.
- Le Registre garde tout ; seule la *poussée* est limitée. Une information qui ne tient pas dans le budget reste lisible mais ne
  s'impose plus.
- Reste : conseils vocaux (`audio/advisor.rs`) et lettres scellées gardent leur propre canal ; la table d'actions du Registre est
  minimale (pas de filtre par rubrique).
