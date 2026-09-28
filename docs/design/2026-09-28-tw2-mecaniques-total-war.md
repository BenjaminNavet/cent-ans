# TW2 — Mécaniques Total War, deuxième passe (28/09)

Suite de `2026-09-24-rapprochement-total-war.md`. Écarts restants avec Total War (Medieval II pour les
mécaniques, TW récents pour l'ergonomie) relevés le 28/09 ; le joueur a validé le plan et l'enchaînement
des lots sans validation intermédiaire.

Principes : toute règle dans `core/`, tout chiffre dans `data/rules/*.json` (schéma dans `data/schemas/`),
l'UI Godot se contente d'afficher et d'émettre des ordres. Nouveaux champs d'état en `serde(default)` ;
pas de changement de `STATE_VERSION` sauf nécessité (alors ADR). L'IA doit savoir utiliser chaque
nouvelle mécanique (sinon le joueur seul en profite).

ADR réservés : **0100** (SB), **0101** (T1), **0102** (T2), **0103** (T3), **0104** (T4), **0105** (T5).
(0098/0099 sont réservés par l'orchestration RS, `docs/wip/restes.md`.)

## SB — Lisibilité et rythme de la destruction en siège (bataille)

Constat : `get_siege().pieces[i].hp/max_hp` existe mais `battle_siege.gd` ne l'affiche pas (noircissement
et abaissement seulement). Rythme mesuré au niveau de fortification 3 : porte 540 PV / bélier 5 PV/s ≈ 1 min 50 ;
pan de mur 3 100 PV / trébuchet 84 PV par tir toutes les 12 s ≈ 7 min 30.

- **Barres de vie** des pièces (porte, pans de mur) façon TW : petite barre flottante enluminée au-dessus
  de la pièce dès qu'elle est endommagée ou visée (bélier au contact, engin qui tire dessus), masquée
  quand elle est intacte et non visée ; couleur par camp du propriétaire (défenseur). Même barre pour le
  bélier et les tours de siège (leur PV = équipage/structure, selon ce que le cœur expose). Info-bulle
  « Porte : 320/540 ». Le cœur expose si la pièce est visée (`under_attack`) plutôt que de le deviner dans
  Godot.
- **Rythme** (données seulement, `data/rules/siege_works.json`) : cibles au niveau 3 — porte tombée en
  **40-60 s** par un bélier à plein équipage ; brèche d'un pan en **6-10 tirs** d'un trébuchet (≈ 1 min 30
  à 2 min), une bombarde plus vite qu'un mangonneau. Niveau 5 nettement plus long (×1,5-2), niveau 0-1
  rapide. Mesurer avant/après par sonde (exemple ou test) et consigner le tableau dans l'ADR 0100.
- Vérifier les tests de siège existants (auto-résolution et batailles de siège) : l'équilibre global
  (taux de prise) ne doit pas basculer ; ajuster si besoin et le dire.

## T1 — Sort de la ville prise (campagne)

Aujourd'hui `siege::capture` remet la ville et ajoute des troubles, sans choix. TW : occuper / piller /
raser. Version Cent Ans :

- **Occuper** : comme aujourd'hui (troubles de capture).
- **Mettre à rançon** (appatis, rançon de ville) : or immédiat modéré, troubles accrus, pas de destruction.
- **Piller** : or élevé, population et richesse amputées, un bâtiment endommagé ou détruit, troubles
  forts, perte de piété/réputation ; les troupes gagnent de l'expérience.
- **Raser / brûler** (villes secondaires et places seulement, jamais une capitale historique) : or faible,
  la place perd un niveau ou devient ruine, gros malus diplomatique.
- Chiffres dans `data/rules/capture.json` (schéma). Ordre `ChooseCaptureOutcome` ; si c'est le joueur, la
  prise ouvre une **décision en attente** (fenêtre enluminée avec les 3-4 choix et leurs effets chiffrés
  via un aperçu du cœur) ; l'IA choisit selon sa doctrine/trésor. Chronique et événement de campagne.
- Tous les appelants de `capture` (siège, marche, agents) passent par le même chemin.

## T2 — Reconstitution des armées et réserves de recrutement (campagne)

- **Reconstitution** : chaque saison, une armée qui ne combat pas regagne des hommes (% des effectifs
  manquants) selon le territoire (propre > allié > neutre > hostile = 0), la posture (campement/garnison
  +, marche forcée 0, siège réduit), l'hiver, les compétences d'intendance du général et les bâtiments
  de la province. Coût en or proportionnel aux hommes rendus. Les unités ramenées à 0 ne reviennent pas.
- **Réserves de recrutement** (Medieval II) : chaque établissement a une réserve par unité recrutable,
  plafonnée, qui se remplit chaque saison (taux et plafond selon bâtiments/niveau) ; recruter consomme
  la réserve. L'UI de recrutement affiche « 2 disponibles, +1 dans 2 saisons ».
- Règles dans un **nouveau module** (`replenish.rs`, `recruit_pool.rs`) et `data/rules/replenishment.json`
  pour éviter les conflits avec le lot RS B (`economy.rs`, `economy.json`). Branchement dans le tour.
- Affichage : taux de reconstitution dans la fiche d'armée (info-bulle détaillant les facteurs).

## T3 — Compagnies de mercenaires (campagne)

Les routiers existent comme unité recrutable en ville (période 1356-1395). TW : réserve régionale de
mercenaires engageables **par l'armée elle-même, n'importe où dans la région**, sans délai.

- Réserve de mercenaires par région (groupe de provinces), alimentée selon l'époque (compagnies après
  Brétigny, Génois arbalétriers, Brabançons, Écossais…) ; unités marquées `mercenary` dans `data/`.
- Ordre `HireMercenary { army, unit }` : coût initial élevé, entretien ×1,5-2, pas de temps de
  recrutement, limite par tour. Les mercenaires impayés (trésor négatif) désertent ou pillent la
  province (troubles).
- Panneau « Mercenaires » depuis la fiche d'armée ; l'IA en engage quand elle est riche et menacée.

## T4 — Points de capture en siège (bataille)

À faire **après SB** (mêmes fichiers).

- Points de victoire TW : la **place du marché** (déjà placée par BR3) et la porte/le donjon ; un point est
  pris quand l'attaquant y a plus d'hommes que le défenseur pendant N secondes (barre de progression).
- Victoire de l'attaquant quand il tient la place centrale un temps donné ; les défenseurs y gagnent un
  bonus de moral (« dernier carré »). Règles dans `data/rules/siege_works.json` ou un fichier dédié.
- Affichage : drapeaux au sol, barre de capture, alerte « La place est menacée ». IA : les défenseurs se
  replient sur la place quand les murs tombent, l'attaquant y converge.

## T5 — Traditions d'armée (campagne + bataille)

- L'**armée** (et non seulement le général) accumule de l'expérience par bataille gagnée/livrée et
  débloque des traditions (1 choix par rang, 3-4 rangs) : marche (+mouvement), intendance
  (+reconstitution T2), tir (+précision), assaut (+siège), tenue (+moral). Données
  `data/rules/army_traditions.json`.
- Nom et bannière d'armée conservés quand le général change ; dissolution = perte des traditions.
- Effets appliqués en bataille via les bonus existants (`RuleValues`) ; panneau « Traditions » dans la
  fiche d'armée ; l'IA choisit selon sa doctrine.

## Plus tard (phase 2, non lancée)

Missions/objectifs de faction ; papauté (Schisme, excommunication, croisade) ; guerre civile et loyauté
des grands (Armagnacs/Bourguignons) ; bataille personnalisée depuis le menu ; mode garde des unités ;
sorties des assiégés ; choix du sort des captifs à la TW.

## Vagues

Session parallèle RS en cours (6 agents) : vagues de **3 agents** au plus.

| Vague | Lots | Remarque |
|---|---|---|
| 1 | SB, T1, T2 | fichiers disjoints (bataille siège / `siege.rs` campagne / nouveaux modules) |
| 2 | T3, T4, T5 | T3 après T2 (recrutement), T4 après SB, T5 après T2 (intendance) |

Suivi : `docs/wip/tw2.md`.
