# ADR 0167 — Voyages maritimes depuis un port

Date : 2026-10-03. Statut : accepté. Lot EM (retour du joueur : « depuis la terre, je ne peux pas
aller dans l'eau et naviguer jusqu'à une destination ; ça devrait être possible depuis un port »).

## Contexte

L'embarquement (M2, ADR 0010 ; routes SL1, ADR 0139) ne reliait que deux ports voisins par **une**
arête maritime du graphe des colonies, et seulement sur un clic droit sur ce port précis. Un clic sur
la mer ne faisait rien (« aucun chemin ») ; un port lointain non relié directement (Londres → Bayonne,
Southampton → La Rochelle) tombait sur la marche terrestre et échouait ; une armée ayant déjà bougé
voyait son embarquement refusé en silence puis une erreur de marche sans rapport.

## Décision

- **Voyage** (`sim-campaign::voyage`) : depuis le port où elle se tient, une armée rejoint en une
  saison tout port atteint par une chaîne d'au plus `max_voyage_legs` arêtes maritimes (donnée
  `data/movement/rules.json`, 4 ; 1 par défaut pour les données anciennes), au moindre coût. Escales
  seulement dans des ports non ennemis ; la destination peut être ennemie (débarquement, comme avant).
- **Ordre `Embark`** inchangé dans sa forme (`to_port`) : chaque traversée du voyage peut être
  interceptée (NV1) et subit son gros temps (SL1) ; l'armée fait escale entre deux traversées. Une
  interception en attente (bataille navale du joueur) la laisse au port de l'escale. Toute la saison,
  comme avant ; les refus sont explicites (`NoSeaRoute`, `EmbarkNeedsFullTurn`).
- **Interface** : clic droit sur un port atteint par la mer → embarquement, sauf si la marche y arrive
  ce tour (un port voisin le long de la côte reste une marche). Clic droit **sur la mer** depuis un
  port → voyage vers le port atteignable le plus proche du point. Survol : tracé du voyage sur les
  routes maritimes, escales et « clic droit pour embarquer » ; sur la mer hors d'un port : « l'armée
  doit d'abord entrer dans un port ». L'armée glisse le long du tracé.
- Pont : `sea_voyage(army, port)`, `sea_port_near(army, x, y)`.

## Conséquences

- Pas de flotte-entité ni d'armée « en mer » entre deux tours (limite de l'ADR 0028 inchangée).
- L'IA garde ses traversées d'une arête (sa planification suit le graphe) ; elle peut emprunter les
  voyages plus tard si l'équilibre le demande.
- Aucun changement de `STATE_VERSION`.
