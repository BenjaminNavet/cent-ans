# ADR 0020 — Relief des champs de bataille

Date : 2026-09-25. Statut : accepté. Lot R2 (suivi : `docs/wip/r2-relief-bataille.md`).

## Contexte

Le joueur trouve les cartes de bataille « trop plates, sans réalisme ». Le champ (1200 × 800 m, grille
de 10 m) n'avait qu'une inclinaison et 3 à 8 collines gaussiennes ; bois et boue étaient des disques.
Le relief est une règle de jeu (vitesse selon la pente, portée des tireurs selon la hauteur, lignes de
vue, choix de position de l'IA) : il doit vivre dans le cœur, rester déterministe et ne pas décaler les
tirages des batailles existantes (unités, météo, siège, site B5).

## Options

- **Relief de rendu seulement** (bruit dans le shader et le maillage) : rien à tester, mais les soldats
  flotteraient ou s'enfonceraient, et le relief vu ne serait pas celui qui compte pour la règle.
- **Remplacer le générateur** : casse la reproductibilité des graines (tous les tirages suivants
  bougent) et les tests existants.
- **Couche de détail dans le cœur, tirée d'un flux dérivé** : l'ancien champ sert de base, ses tirages
  sont inchangés ; tout le reste vient de `BattleRng::derive(RELIEF_STREAM)` (même principe que B5).

Pour les bois et la boue : contour bruité par zone (nouveau type, pont et rendu à adapter, `Zone`
cesse d'être un disque) ou **grappes de disques** (massif de lobes qui se chevauchent + bosquets).

## Décision

- Couche de détail dans le cœur (`core/crates/sim-battle/src/relief.rs`), flux dérivé, grille de 10 m
  conservée : fBm à déformation de domaine, bruit « ridged » (crêtes, croupes) en collines et
  montagnes, 1-2 vallons qui drainent vers la rivière, ruptures de pente (escarpements ; en plaine,
  lande et bocage, volées de talus parallèles — les « rideaux » picards), micro-ondulations, plaine
  alluviale le long de la rivière. Styles et amplitudes par terrain (`ReliefStyle::of`).
- Jouabilité : pente bornée au centre des deux lignes de déploiement (relaxation pondérée qui garde
  la hauteur moyenne) ; en collines/montagnes, la ligne du défenseur reste au moins 8/16 m au-dessus
  de celle de l'attaquant (rampe douce si besoin).
- Bois et boue en grappes de disques : le disque d'ancrage garde son indice dans `forests` / `mud`
  (rétréci, déplacé vers son meilleur terrain : bois sur les pentes et crêtes, boue au creux), lobes et
  bosquets dans `forest_parts` / `mud_parts` (serde par défaut). `Zone` et l'API (`in_forest`,
  `in_mud`, `height`…) ne changent pas ; les mares B5 suivent la boue, donc les creux.
- Rendu : le détail sous 10 m (normales, couleurs selon pente/creux/crête, rochers) reste dans le
  shader de sol, sans effet sur la règle.

## Conséquences

- Mêmes graines : mêmes unités, météo, siège et tirages suivants (test sur l'état du flux après le
  champ) ; en revanche relief, bois et boue d'une graine donnée changent (les résultats des batailles
  aussi, légèrement).
- Pentes plus fréquentes : la vitesse et la portée en dépendent déjà ; l'IA ne lit le relief que par
  la hauteur (point ouvert : lire les vallons et les crêtes).
- `get_terrain` exporte bois et boue comme la réunion des ancres et des parties ; le rendu évite de
  compter deux fois les arbres dans les recouvrements.
