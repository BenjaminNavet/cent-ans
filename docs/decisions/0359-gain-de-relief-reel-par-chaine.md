# 0359 — Gain de relief réel par chaîne (remplace le mur de crête) et lacs à l'altitude réelle

Statut : accepté (14/10, lot CC CS). Complète l'ADR 0355 (addendum CD, relief réel déformé) et l'ADR 0353.

## Contexte
`compose-relief-real` relevait une bande étroite le long des crêtes d'auteur à un PLANCHER (`crest`). Là où le
MNT réel est plus modéré que la chaîne d'auteur, cela fabrique un mur lisse qui ignore la vraie topographie :
- Apennin (chain_05) : un mur de 9 px, le massif réel (1500-2900 m) reste plat autour ;
- Carpates (chain_09) : un « tube » lisse décalé de l'arc réel, les vraies montagnes (texturées) à côté, d'où
  l'aspect « fondu » ;
- Tuz Gölü, Beyşehir… : le niveau de lac d'auteur (10e centile de la rive du relief d'AUTEUR, bas en plaine)
  creusait des cuvettes plates de 850 m de profondeur sur le plateau anatolien (le « rectangle » près de Konya).

## Décision
1. `relief_real.json` > `chains.<id>` : `crest: false` supprime le plancher de la chaîne ; `gain`, `socle_m`,
   `span_m`, `core_px`, `influence_px` amplifient le relief RÉEL : `h' = h + (g-1) x clip(h - socle, 0, span) x w`,
   `w` = 1 à moins de `core_px` de la polyligne, fondu smoothstep jusqu'à `influence_px`. La forme (vallées,
   arêtes) vient du MNT ; seule l'amplitude croît, bornée par `span` (pas de pics aberrants), la plaine sous le
   socle est intacte. Appliqué à la résolution de la carte (avant cols, fleuves, plateformes) et à chaque
   fenêtre fine (mêmes formules). Réglages : Apennin g 1,8 / socle 350 m / span 1100 m / cœur 22 px /
   influence 52 px (sommets ~ 3 500 m, exagération RE ensuite) ; Carpates g 1,2.
2. Niveau des lacs intérieurs = `max(auteur, 10e centile de la rive réelle - 1 m)`.
3. `refine.min_real_coverage` 0,5 -> 0,95 : on ne raffine E2/E3 que là où la pyramide réelle fine est
   (quasi) complète, pas de raccord avec trous comblés par l'étage grossier.
4. Mémoire et sécurité : cadre complet évité (`inside.npy`), contraintes en place, écriture dans `*.new` puis
   échange par renommage, `--bbox` (région, sortie dans le dossier de travail, rien de partagé touché).

## Conséquences
- Les cuissons dépendantes (`geo splat`, `geo colormap`, `geo navgrid`) sont à rejouer après chaque changement.
- Autres chaînes : même mécanisme, à ajouter une par une dans `chains` si le joueur le demande.
