# 0160 — Armées sur la carte : cible survolée, ost hors de la ville, échelle sous-linéaire

Date : 2026-10-02 · Lot SA (`docs/wip/sa-selection-armees.md`)

## Contexte

Retour du joueur au lancement d'une campagne France :

1. cliquer sur l'ost du roi sélectionne Paris ;
2. rien n'indique ce que le clic va prendre ;
3. les modèles d'armée sont peu lisibles et leur taille « fait bizarre » au zoom.

Causes mesurées (captures 24 / 60 / 150 / 400) :

- l'ost en garnison est posé sur la ville ; son échelle suit la distance caméra (`0,014 × d`,
  taille écran constante), donc à 400 il couvre Paris et sa région. L'écu de la ville lui est
  collé. Le picking de l'armée tient en deux points de 26 px, celui de la ville en un disque
  entier : la ville gagne presque toujours ;
- aucun retour de survol sur les armées ni sur les villes (seule la province s'éclaire) ;
- une figurine qui garde sa taille écran grossit de 45× en taille monde entre le zoom le plus
  proche et le parchemin : le paysage rétrécit sous elle, à l'inverse d'un objet posé.

Référence : Total War (Warhammer 3, Three Kingdoms). Le personnage a une taille monde fixe,
héroïque (à peu près celle d'une ville) ; il rétrécit donc à l'écran quand on dézoome. Ce qui
garde une taille écran constante est la bannière 2D au-dessus de lui (écu, effectif). Le
survol éclaire le personnage et sa bannière ; le personnage prime sur la ville sous le curseur.

## Décision

1. **Une seule fonction de visée** (`CampaignMap.pick_target`) sert au survol et au clic : ce
   qui est éclairé est ce que le clic prend. Une armée visée en plein (silhouette projetée ou
   plaque) prime sur la ville ; hors silhouette, le plus proche gagne. Un second clic au même
   endroit alterne toujours armée / ville (Q2 conservé).
2. **Surbrillance de survol** : armée — socle éclairci, figurines éclaircies (`highlight` du
   shader), plaque à liseré doré ; ville — écu agrandi à halo clair ; curseur « main ». La
   province ne s'éclaire plus quand un objet est visé (une seule cible à la fois).
3. **L'ost se tient à côté de la ville** : écart = rayon de la ville + demi-emprise de l'ost à
   l'échelle courante, côté sud-est. L'écart suit l'échelle, donc l'ost ne recouvre la ville à
   aucun zoom. Pendant une marche animée, l'écart est nul (le trajet réel est montré).
4. **Échelle sous-linéaire** : `échelle = MIN_SCALE × (d / d₀)^p` avec `p = 0,65` (réglage
   `map.army_scale_exponent` de `data/ui/campaign_map.json`) au lieu de `p = 1`. L'ost rétrécit
   à l'écran en dézoomant (comme un objet posé), sans devenir illisible : à 400 il fait 35 % de
   sa taille écran d'avant. La plaque d'effectif garde sa taille écran : c'est elle le repère au
   loin, comme la bannière de Total War. Sur le parchemin (poids stratégique), l'étendard
   retrouve l'ancienne loi linéaire (pion lisible, figurines fondues).
5. **Socle** : l'anneau au sol devient un socle de pion (disque teinté à la couleur de la
   faction, liseré), plus lisible que l'anneau à 50 % d'opacité.

## Conséquences

- Les armées occupent moins d'écran en vue régionale ; la lecture passe par la plaque et le
  socle. Réglage unique (`army_scale_exponent`, 1 = ancien comportement) si le joueur juge
  l'ost trop petit.
- `ArmyMarkers.pick_screen_scored` renvoie aussi `direct` ; `SettlementLayer` gagne
  `set_hovered`. Aucune règle de jeu touchée (`core/` intact).
- La position affichée d'une armée en garnison n'est plus celle de la colonie : l'écart est
  purement visuel (les ordres et distances lisent la simulation).
