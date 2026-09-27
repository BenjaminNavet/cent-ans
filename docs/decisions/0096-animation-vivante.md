# ADR 0096 — Animation vivante (AN1)

Statut : accepté (27/09/2026). Orchestration : `docs/wip/an1-animation-vivante.md`.
Contexte commun : les figurines de bataille sont animées par textures d'os cuites (lot V2,
~45 clips) ; rien ne bougeait en dehors du squelette (tissus, crins, bannières raides à part
l'onde des étendards d'EP5). Bible DA § 6.

## A. Mouvement secondaire en shader de sommets (lot AN1a)

### Décision
Un mouvement secondaire bon marché est ajouté **dans le shader de sommets**, après le skinning,
dans le repère posé de la figurine (+Z = avant) : aucun os, aucune simulation CPU, **aucune
recuisson** des figurines.

- **Pièces souples reconnues avec les données déjà cuites** (analyse des maillages fins) :
  - bas des surcots, jaques et cottes (codes livrée, étoffe, gambison) : poids = part des os
    des jambes du sommet (0 à la taille, ~0,85 à l'ourlet, gradient du transfert de poids) ; les
    chausses, entièrement sur les jambes (≥ 0,99), restent rigides ;
  - caparaçon : code armoiries porté par les os du cheval (`sever_bones[0]`), poids selon la
    hauteur de repos (ourlet 0,48 m → selle 1,45 m, données) ;
  - queue : os `Tail1-7`, poids selon le rang dans la chaîne ;
  - crinière : robe du cheval à la teinte exacte des crins (`HAIR_RGB` 0,33 → 84/255 en couleur
    de sommet 8 bits).
- **Moteurs** : vitesse du régiment (`BattleSoldiers._speed`, uniforme `move_speed` envoyé
  seulement quand il change, arrondi à 0,1 m/s) → recul vers l'arrière ; vent de la bataille
  (direction et force de `data/fx/battle_finish.json` `wind`, le même que l'herbe et les
  drapeaux) et rafales ; flottement (deux sinus, phase selon la position de repos et le soldat).
- **Pas de pénétration** : la poussée est pondérée par l'orientation de la face (les faces qui
  regardent la poussée s'évasent, celles qui lui font face restent plaquées sur les jambes ou
  les flancs) ; un « évasement » soulève le bord libre.
- **Étendards** (`battle_standard_flag.gdshader`) : amplitude et vitesse de l'onde lues dans les
  données, ondulation le long de la hampe, et **vent apparent** de l'allure du porteur (clip du
  jeu : arrêt, marche, course, sonnerie → l'étoffe traîne vers l'arrière quand il court).
- **Données** : `data/fx/atmosphere.json` `secondary_motion` (schéma
  `fx_atmosphere.schema.json`) : amplitudes par pièce (recul, vent, flottement, fréquence,
  évasement), échelle du vent, distance maximale (160 m, fondu dès 70 %), vitesse de
  référence. Uniformes posés par `game/scripts/battle/battle_secondary_motion.gd`.
- **Portée** : LOD0/LOD1/LOD2 fins et figurines Quaternius (`--coarse-figures`, mêmes rigs et
  codes) ; figurines rigides `--legacy-figures` inchangées (autre shader) ; éteint sur les
  couches de poses figées (cadavres, blessés, renversés, porte-étendard tombé). `--no-an1a`
  coupe tout (banc A/B).

### Alternatives écartées
- **Canal de poids cuit** (`battle_fine.py`) : plus précis (vraie distance à l'attache), mais
  recuisson complète ~45 min et conflit avec AN1b (textures d'os) ; les poids déduits suffisent.
- **Os de tissu dans les clips** : coût de cuisson et de texture d'os, mouvement répétitif lié au
  clip, pas de vent.
- **Simulation de tissu** (Godot SoftBody) : incompatible avec le rendu par MultiMesh de milliers
  de soldats.

### Conséquences
- Coût GPU mesuré au banc FG5 (voir `docs/wip/an1a-mouvement-secondaire.md`).
- ~7 sommets du corps du cheval fin partagent la teinte des crins et bougent avec la crinière
  (quelques millimètres) : invisible ; à retirer si une recuisson ajoute un vrai masque.
- Le mouvement utilise `TIME` (comme les drapeaux) : il continue pendant la pause.
- La vitesse est celle du régiment, pas du soldat (soldats qui se replacent : pas de recul).

## B. Nouveaux clips (lot AN1b)

À rédiger par le lot AN1b.
