# Lot SA — sélection, survol et échelle des armées (carte de campagne)

Branche `feat/sa`, worktree `../gp-sa`. ADR 0160. Rendu et entrées seulement (`core/` intact).

Demande (2026-10-02) : cliquer l'ost sélectionne Paris ; pas de surbrillance de ce qu'on vise ;
modèles d'armée médiocres et changement de taille bizarre au zoom ; s'inspirer de Total War.

## Lots
- [ ] SA1 visée unique (`CampaignMap.pick_target`) : armée visée en plein > ville ; silhouette
      projetée au lieu de deux points de 26 px.
- [ ] SA2 survol : armée (socle, figurines, plaque), ville (écu), curseur main, province éteinte
      quand un objet est visé.
- [ ] SA3 ost à côté de la ville (écart qui suit l'échelle).
- [ ] SA4 échelle sous-linéaire (`map.army_scale_exponent`), ancienne loi sur le parchemin.
- [ ] SA5 socle de pion.
- [ ] Test `game/tests/sa_pick_test.gd` + smoke ; capture de contrôle 24/60/150/400.

## État
Squelette : ADR et note. Rien d'implémenté.

## Prochaine étape
SA4 + SA3 dans `army_markers.gd` / `army_marker.gd`, puis SA1/SA2.
