# XXXX — IA-CAV : charge groupée de la cavalerie (boucle juge 2)

## Contexte
Mesures du lot COMBAT (ADR 0792, IA contre IA, 8 graines, `tests/combat/matchups.rs`) : 10 chevaliers contre 10 arcs
longs 0/8 (333 pertes contre 50), armée française mixte contre 6 arcs + 4 hommes d'armes 0/8, 8 chevaliers contre
8 hommes d'armes 8/8 pour 4 pertes, premier contact vers 250 s. Les leviers de choc, moral, vitesse et munitions
cassent `ai_tactics` (Crécy) : la correction est dans l'IA.

Lecture des traces : l'archer défenseur plante ses pieux, donc aucune cible n'est « isolée » au sens de l'ancienne
règle (pas de pieux de face) ; les escadrons avancent au pas ralenti (x0,45 tant que l'écart dépasse 375 m) et
entrent un à un dans la portée des arcs. L'état `Charging` n'existe qu'à moins de 120 m (`charge_distance_cavalry_m`).

## Décision
- `ai/mass.rs` + `data/rules/battle_mass_charge.json` (schéma `battle_mass_charge_rules.schema.json`) : les escadrons
  libres d'un camp qui a l'initiative (ou dont un escadron est déjà sous les traits) visent la grappe de tireurs
  isolés la plus proche (aucune troupe de mêlée à moins de `isolation_distance`), s'alignent au pas hors de portée
  (portée effective + `stage_margin_m` = 20 m), puis partent ensemble au galop quand tous sont en place, qu'un
  escadron est touché ou qu'un autre est déjà lancé. Sans état : tout est recalculé à chaque pas de l'IA.
- Pieux de face : charge en tenaille, chaque escadron rejoint l'extrémité de la ligne de son côté, à 75 degrés de
  son front (hors de l'arc frontal et des pieux), sans traverser le front ; sans pieux, charge frontale groupée.
- Réservé aux armées de cavalerie seule : une armée qui a de l'infanterie garde l'ancienne logique (EQ7, la cavalerie
  attend son infanterie). Essayé avec infanterie : la bataille symétrique `symmetric_flat_battle_is_open` tombait à
  0/10 pour l'assaillant.
- Un groupe plus faible que la grappe (`min_power_ratio`) ne s'aligne pas, sauf s'il est déjà sous les traits.
- Cavalerie seule (sans infanterie à attendre), camp offensif : le poste d'aile est rejoint au trot, pas au pas.
- Hors périmètre, essayé et retiré : faire attendre l'infanterie à pied à l'arrêt face à la charge (n'arc-boute pas
  assez : 275 pertes contre 234).

## Mesures (8 graines, avant -> après)
- 10 chevaliers / 10 arcs longs : 0/8 (333 contre 50) -> 5/8 (275 contre 100).
- Armée française mixte / 6 arcs + 4 hommes d'armes : 0/8 (460 contre 52) -> 0/8 (461 contre 52), inchangée (a de l'infanterie) ; Crécy reste anglais.
- 4 chevaliers / 20 milices : 3/8 (73 contre 514) -> 1/8 (67 contre 250) (trot vers le poste d'aile).
- 8 chevaliers / 8 hommes d'armes : 8/8 (4 contre 231) -> 8/8 (6 contre 234) ; premier contact 250 s -> 162 s.
- Autres matchups inchangés.

## Conséquences
- Aucun test à graine existant n'a dû être ajusté ; bandes `matchups::outcome_bands` mises à jour (10K/10L : 1 à 5 victoires).
- Ouvert : des hommes d'armes en ordre restent balayés par une charge frontale égale (modèle de choc ADR 0711 :
  piste = poids/armure du choc, hors IA).
