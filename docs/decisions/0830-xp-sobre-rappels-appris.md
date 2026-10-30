# XP-SOBRE : un rappel appris ne se répète plus

Statut : accepté (30/10/2026, lot D de la boucle juge 2, itération 3).

## Contexte

Le juge (constats M3, M4) relève que l'Audience répète à l'identique, saison après saison, des rappels
que le joueur a déjà vus (`free_slot`, `advice`, `idle_army`, et l'Affaire « choisir 2 ambitions » 10
saisons sur 10 pour les 8 maisons), que le score de règne ne dit rien avant la saison 10, et que
certaines phrases sortent vides (« : . ») quand la liste des raisons du trésorier est vide.

## Décision

1. **Mémoire des rappels dans l'état de campagne** (`sim_campaign::reminders::ReminderLedger`, champ
   `reminders`, absent des anciennes sauvegardes, sans changement de `STATE_VERSION`). `agenda()` reste
   une fonction pure de l'état ; c'est `game_api::note_season_end` (appelé par le client juste avant la
   fin de saison, sur le fil de calcul du tour) qui compte les affichages. Après `learn_after` saisons
   où la cause reste vraie (2), le rappel se tait `snooze_seasons` saisons (4). Une cause intermittente
   garde son compteur (seuls le silence ou le rappel le remettent à zéro). `game_api::recall_reminder` le rappelle (bouton « Rappeler »).
2. **Cadence sans mémoire** pour `ambition_choice` : `push_first_seasons` (2) puis `push_every_seasons`
   (4), c'est-à-dire saisons 0, 1, 4, 8, 12.
3. **Jalon de règne** (`game_api::progress::reign_milestone`) aux saisons de `milestones` (4, 8, 12) :
   score d'acte contre le score attendu (données), « en avance / à l'heure / en retard » avec les deux
   objectifs les plus lourds restants.
4. **Textes** : les raisons du trésorier sont les lignes projetées du budget (`top_terms`, somme égale
   au solde net, dernière case « autres éléments » seulement si le reste est non nul) ; une liste vide
   utilise la ligne `treasury_swing_plain`, jamais « : . ».

Tout est réglé dans `data/rules/agenda.json` (schéma `agenda_rules.schema.json`) et
`data/ui/audience.fr.json`.

## Conséquences

- Un joueur qui ignore une consigne n'est plus relancé chaque saison ; le rappel revient après un silence.
- Le client appelle `note_season_end` avant `end_turn_with` ; sans cet appel (tests, bots) rien ne change.
- Le bouton « Rappeler » côté client reste à câbler (point ouvert, panneaux livres en attente d'XP-R3b).
