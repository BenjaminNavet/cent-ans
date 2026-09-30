# 0146 — Sommation de paix du suzerain au joueur

## Contexte

Riazan, joué, déclare la guerre à Moscou pour reprendre Kolomna (objectif de son titre). Les deux sont
vassaux directs de la Horde d'Or : la guerre est privée (FE § 4.3.5) et le seigneur IA, bien plus fort,
**imposait la paix dans le même appel** que la déclaration. L'interface annonçait « La guerre est
déclarée » puis faisait marcher l'armée, qui entrait dans Kolomna comme en temps de paix : ni siège,
ni garnison à combattre. Riazan ne pouvait jamais attaquer Moscou tant que la Horde restait suzeraine.

## Décision

1. Quand le seigneur IA commun rend le verdict « imposer la paix » et que l'**attaquant est le joueur**,
   la guerre commence et le joueur reçoit une offre féodale `Proposal::PeaceSummons { target }`
   (type d'appel `summons`) : **Obéir** = paix imposée (trêve, loyauté −`imposed_peace_loyalty_drop`) ;
   **Passer outre** (ou laisser expirer) = la guerre continue, loyauté −`defied_summons_loyalty_drop`
   (`data/rules/feudal.json`, 15).
2. Entre IA, et pour le joueur cible d'une guerre privée, l'arbitrage reste immédiat (inchangé).
3. L'aperçu d'escalade de la déclaration dit « vous sommera de faire la paix ».
4. Garde côté carte : si la guerre n'est pas en place après la déclaration, l'armée ne marche pas et
   un message renvoie au journal.

## Conséquences

- Le joueur garde la main sur une guerre qu'il vient de confirmer ; défier le suzerain a un coût.
- Une troisième sorte d'appel féodal (`summons`) dans `get_feudal_offers`, le panneau diplomatique et
  l'arbre féodal.
