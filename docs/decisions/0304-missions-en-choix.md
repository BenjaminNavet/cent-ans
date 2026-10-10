# 0304 — Les missions sont proposées en choix (offre de 2 à 3 candidates)

Statut : accepté (lot WR turn, suite de WH turn).

## Contexte
Jusqu'ici (ADR 0127, 0207, 0281) le cœur imposait une mission par tour libre : le joueur la subissait.
Warhammer III propose au contraire des dilemmes : plusieurs missions candidates, ou aucune.

## Décision
- `MissionsState.offer` (`#[serde(default)]`, absent des anciennes sauvegardes) porte au plus **une offre** :
  jusqu'à `offer_candidates` (3) missions complètes, de gabarits **distincts**, tirées par poids parmi celles que le
  générateur sait faire (mêmes filtres : faction, chaîne `after`, cible plausible) avec le même tirage déterministe.
- Ordre `Order::ChooseMission { choice: Option<usize> }` : `Some(i)` prend la candidate `i` (son id et son horloge
  démarrent à l'acceptation : délai conservé, base des traités recalculée, compteurs à zéro), `None` refuse.
  Refusée si aucune offre, indice inconnu, ou deux missions déjà actives (l'offre reste alors ouverte).
- Une offre ignorée **lapse** au bout de `offer_expiry_turns` (2) tours : avis `expired`, le délai de carence
  `offer_cooldown_turns` s'applique comme pour un refus ou une clôture. Pas de nouvelle offre tant qu'une attend.
- Valeurs (`offer_candidates`, `offer_expiry_turns`, et la fréquence par `offer_cooldown_turns`) dans
  `data/missions.json`, validées par `missions.schema.json`.
- IA : `missions::ai_choose_mission` choisit la candidate à la meilleure valeur (or + 20 par prestige + 10 par point
  d'ordre + 100 pour une compagnie), à égalité le délai le plus court ; branchée dans le planificateur pour la
  faction du joueur quand elle est pilotée par l'IA. Les autres factions n'ont pas de missions (inchangé).
- Pont : `get_mission_offer`, `choose_mission(choix)` (négatif : refus). UI : `MissionOfferController` réutilise
  `ChronicleWindow` (comme le sort d'une place prise) : un bouton par mission (objectif, délai, récompense) et
  « Refuser toutes les missions » ; « Plus tard » laisse l'offre courir jusqu'à son expiration.
- Filtre du journal par genre (purement visuel) : regroupement des `kind` d'événements dans
  `data/ui/journal_genres.json`, barre de boutons dans `JournalView`, choix retenu pour la session.

## Conséquences
- Plus aucune mission n'est active sans acceptation : l'équilibre du nombre de missions dépend du joueur.
- Une candidate peut devenir caduque entre l'offre et l'acceptation (province déjà prise) : elle réussit alors
  au tour suivant ; accepté comme cas marginal.
