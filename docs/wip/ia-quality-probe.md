# Sonde de qualité de l'IA de campagne (`ia_quality_probe`)

Branche `feat/ia`. Exemple `core/crates/ai/examples/ia_quality_probe.rs` : mesure la qualité du
jeu de l'IA (prises manquées, pertes sans secours, armées oisives, batailles suicidaires,
fragmentation, sièges, trésor, coût par tour), IA contre IA depuis 1337. Mesure seule : aucune
règle ni IA modifiée. Méthodes et proxys : commentaire de tête du fichier.

## État
- [x] Squelette, métriques, exemples groupés (`VERBOSE=1`)
- [x] Mesure graines 1-2, 60 tours (342 s, machine chargée)

## Résultats (60 tours, moyenne graines 1-2)
Prises manquées 78/graine (1,3/tour), pertes sans secours 7 (sur 25,5 places perdues sous un
siège déjà vu), 20 % d'armées oisives en guerre, 3 batailles suicidaires (sur 13,5 perdues par
l'attaquant), 0,1 fragment/faction, sièges 50 lancés / 26 pris / 21 abandonnés (5,1 tours par
prise), trésor négatif 7,9 % et thésaurisation 4,9 % des tours-factions.

## Pistes (non traitées ici)
- Petits royaumes (Bosnie, Hum, Cilicie, Calatrava) jamais attaqués : filtre F4 `last_bastions`
  (`campaign.rs` ~1853) ; armées oisives 15-25 tours devant une place vide.
- Pertes sans secours : l'armée reste sur sa propre place menacée (Défense, valeur / (1+étapes)).
- Batailles suicidaires surtout par marche (siège choisi sans compter les armées de campagne voisines).

## Prochaine étape
Aucune pour la sonde ; s'en servir pour mesurer les correctifs de `plan_armies`.
