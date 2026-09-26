# 0070 — Blessés au sol et fuyards désarmés (lot EP12)

Date : 26/09/2026. Statut : accepté.

## Contexte
Le joueur veut une déroute lisible et un champ de bataille vivant : des soldats tombés qui ne
meurent pas sur le coup (ils rampent, s'assoient, s'agenouillent), et des régiments en déroute
qui jettent armes et boucliers. Le rendu des figurines est cuit (texture d'os VAT, lot V2) :
tout clip passe par le kit Blender, et le rendu est fait de MultiMesh (aucun squelette moteur).
Le cœur (`sim-battle`) retire les soldats tués d'un régiment ; il ne distingue pas blessés et
morts, et la campagne compte les blessés rétablis de façon agrégée
(`medicine::recovered_wounded`, après la bataille).

## Décision
1. **Blessé ou tué : tirage déterministe côté rendu, pas de règle du cœur.** Être blessé au sol
   n'a aucun effet de jeu en bataille (le soldat est hors de combat dans les deux cas) ; les
   blessés récupérables sont déjà une règle agrégée de la campagne, indépendante des figurines.
   Ajouter un état par soldat au cœur coûterait mémoire et digests (`b6.rs`) sans rien changer au
   jeu. Le rendu tire donc, pour chaque perte, un hachage de (id du régiment, effectif restant
   dans la simulation, rang dans le lot de pertes) comparé à une part par cause
   (`wounded.share` de `data/fx/battle_gore.json`). La clé vient de l'état de la simulation et
   non d'un compteur du rendu : le rejeu (EP13, ADR 0072) retrouve les mêmes blessés, y compris
   après un saut (les couches sont purgées avec les figurines puis refaites).
2. **Blessés = cadavres d'une autre couche.** Trois clips non bouclés (`crawl` 7 s : tombe sur le
   ventre puis se traîne 2,5 m en ralentissant ; `wounded_sit` 6 s : assis, se tient le ventre,
   se balance puis bascule sur le dos ; `wounded_kneel` 6 s : à genoux, courbé, puis tombe sur
   le côté). Le blessé rejoint les cellules de cadavres (LOD, masquage au loin, plafond
   `corpses.max_total`) dans une couche `…/wounded` dont le jeu de clips est celui des blessés ;
   à la fin du clip il reste immobile, il ne coûte alors pas plus qu'un mort. Le rampant tourne
   le dos à son tueur. Plafonds : `wounded.max_total` (1 500) et `wounded.max_animated` (300
   blessés tombés depuis moins de `animated_seconds`) ; au-delà, mort ordinaire.
3. **Armes jetées : drapeau de face, pas de nouveaux maillages.** Le kit Blender marque les
   armes tenues et boucliers d'un bit du masque de variante (`HELD_MASK = 64`, bits 0-5 =
   variantes, bit 7 = pavois BV3). Le shader les masque quand le régiment fuit (`drop_arms`) et,
   pour un mort ou un blessé, quand le code de `INSTANCE_CUSTOM.w` porte le drapeau 8
   (`CODE_UNARMED`, cumulable avec les codes 1-6 des démembrements). Pas de squelette ni de
   maillage de plus par figurine.
4. **Course de fuite distincte** : clips `flee` / `flee_m` (sur `Run` : buste penché, bras
   écartés mains vides, regard par-dessus l'épaule), jeu `routing` des styles à pied ; repli
   sur `run` si le manifeste n'a pas les clips.
5. **Objets au sol** (`BattleDroppedArms`) : un MultiMesh par sorte (épée, bouclier à la livrée,
   arme d'hast, arc, arbalète), maillages de quelques boîtes bâtis au chargement, tampon
   circulaire `dropped_arms.max_per_kind` (500) ; une débandade en pose au plus
   `max_per_rout` (80) ; masqués au-delà de `far_m` (220 m). Chaque blessé laisse aussi son arme.
6. Les cavaliers ne sont pas concernés (rig et maillages du cheval inchangés).

## Conséquences
- `human.bones.bin` régénéré par Blender (`--only` des 15 figurines à pied) : 5 clips ajoutés à
  la fin (lignes 1 525 à 2 100), les clips antérieurs identiques octet pour octet ;
  `cavalry.bones.bin` inchangé. 45 clips sur 48 possibles dans le shader.
- Banc 15 000 soldats (`--units=63`, machine chargée) : aucun écart mesurable (≈ 32 i/s avant
  comme après à 300 s ; +15 appels de dessin ; 196 blessés, 9 régiments désarmés, 806 armes).
- `--no-ep12` coupe le lot (mesures A/B) ; `--ep12-shot=<wounded|rout>` et
  `tests/ep12_shot.gd` pour les captures.
- Limites : les armes au sol ne suivent pas le relief en pente (à plat) ; un régiment rallié
  « ramasse » ses armes ; les figurines rigides de repli (`--rigid-figures`) n'ont pas les clips.
