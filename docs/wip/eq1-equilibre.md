# EQ1 — équilibre (trouble, Angleterre, banqueroutes, couleuvriniers)

Branche : `worktree-agent-a6c791c0cae04ee1a` (C4 + C5 + DP1 + main fusionnés).

## Outils
- `balance_probe campaign 200 1-8` : nouvelles lignes EQ1 (trouble moyen tous les 10 tours,
  révoltes hors passage aux rebelles, trouble 4 / 1 tour avant la révolte, part des révoltes
  soudaines et en province occupée, banqueroutes / faction / décennie), détail JSON par révolte.
- `start_economy_probe [tours] [factions…]` : économie d'ouverture d'un joueur inactif (trésor,
  revenu, chaque ligne d'entretien, impôt et garnison par province) ; `AI=<graine> STEP=<n>` :
  faction jouée par l'IA, avec la décomposition des tours où le trésor passe sous 0.

## Diagnostic (mesures de départ, C5R + DP1 + main)
- Révoltes : 13,9 par partie hors passage aux rebelles, trouble 71 quatre tours avant, 81 au
  moment (donc pas des pics : le trouble est durablement haut) ; mais **la même province se
  révolte à chaque saison** (Boulonnais 9 fois, Savoie 10, Biscaye 12) car le compteur de saisons
  n'est pas remis à zéro après une révolte ; 45 % en province occupée ; impôt Haut dans 70 % des cas.
- **Le panneau de province affiche `province.unrest`** (troubles de prise et de pillage, qui ne
  nourrissent pas les révoltes), pas le mécontentement des classes qui les déclenche : pour le
  joueur, la révolte tombe d'un chiffre bas → « pic soudain ».
- Angleterre : **embargo en double** au départ (la Flandre « impose » aussi un embargo à
  l'Angleterre dans `fac_flanders.json`, alors que c'est l'embargo anglais) : −11 % de revenu
  (−8 % subi, −3 % imposé) ≈ −1 900 ₶ ; garnisons 48 % de l'impôt, administration 31 %.
  Solde −940 au tour 0, −2 225 au tour 6 (le revenu baisse de 12 % en 6 tours pour tous : la
  richesse de départ converge vers sa cible).
- Banqueroutes : `fac_rebels` compte (seed 1 : 323 saisons) ; mineures (Suisses 280, Grenade 109,
  Gueldre) : leur trésor passe sous 0 par des **événements génériques à coût fixe** (−1 000 à
  −2 500 ₶, brigands, incendie, banquet…) pour un revenu de 400-600 ₶ / saison.
- Couleuvriniers : exigent `bld_armoury`, qui n'est **jamais construit** (améliore
  `bld_muster_field`, jamais construit non plus) → ordres refusés.

## État
- [x] Mesures de départ (balance_probe 8 × 200, century_probe 5 × 464, release)
- [ ] 1. Trouble et révoltes (cible : 15-35 de moyenne, 4-10 révoltes / partie)
- [ ] 2. Angleterre en déficit au tour 1
- [ ] 3. Banqueroutes (< 0,5 / faction / décennie)
- [ ] 4. Couleuvriniers (recrutés après 1380 dans ≥ 1 partie sur 2)

## Prochaine étape
Règles : remise à zéro du compteur après une révolte, troubles de province dans la cible,
affichage du vrai mécontentement ; données : embargo flamand, coût des événements à l'échelle
du revenu, prérequis des couleuvriniers.
