# ADR 0072 — Rejeu d'après bataille

Date : 2026-09-26. Statut : accepté. Lot EP13 (suivi : `docs/wip/ep13-rejeu.md`), backlog Total War
(`docs/audit/backlog-tw.md` : « relecture (replay) »).

## Contexte

Le joueur veut revoir une bataille entière après coup, comme dans Total War : depuis l'écran de fin et
depuis un menu listant les dernières batailles ; lecture, pause, ×1 à ×8, barre de temps avec saut,
caméra libre, sans pouvoir donner d'ordre.

## Décision

### Re-simuler plutôt que filmer

La simulation de bataille (`sim-battle`) est déterministe : même départ, mêmes entrées aux mêmes pas
⇒ même bataille. Un rejeu enregistre donc **le départ** et **les entrées horodatées par pas**, puis
re-simule (`core/crates/sim-battle/src/replay.rs`) :

- `ReplayStart` : `BattleSetup`, graine, échelle effectivement retenue (palier par effectif, palier
  forcé ou carte historique), carte historique éventuelle et s'il s'agit de la bataille historique
  scénarisée ou d'une bataille de campagne sur le site. `ReplayStart::build` est désormais **le seul
  chemin de construction** du pont (`BattleSim.setup`, `setup_historical`) : une bataille et son rejeu
  partent forcément du même état.
- `ReplayAction` : toute entrée du pont qui change la bataille — heure de début, IA d'un camp,
  déploiement (ouverture, placement, lancement), commande du joueur (y compris refusée : elle l'est
  de nouveau), mise à feu et points de muraille de débogage. Chaque entrée porte le pas (`tick`) où
  elle a été donnée ; `tick(dt)` n'exécute que des pas entiers, donc l'horodatage par pas est exact
  quel que soit le rythme d'images ou la vitesse.
- Mesures (`tests/ep13_replay.rs`, sonde `measure`) : démo 1337 (14 régiments, 337 s) re-simulée en
  0,08 s ; Crécy (76 régiments, 907 s) en 0,49 s ; Azincourt (56 régiments, 844 s) en 0,31 s, soit
  1 800 à 4 000 fois le temps réel. Fichiers de 12 à 83 Ko (JSON).

Vérifié : rejouer donne le même `BattleOutcome`, le même résumé B6 (fin, vainqueur, effectif de chaque
régiment), le même journal et la même empreinte d'état, sur plusieurs graines, pour une bataille de
campagne, une bataille sur un site historique et une bataille historique scénarisée.

### Détecter un changement de règles

Les règles évoluent entre l'enregistrement et la lecture. Plutôt qu'un numéro de version des règles
qu'il faudrait penser à incrémenter, l'enregistrement garde des **empreintes d'état**
(`state_digest` : FNV-1a du pas, du tirage aléatoire suivant et, pour chaque régiment, position,
orientation, effectif, moral, fatigue, munitions, état) : au départ, toutes les
`checkpoint_seconds` (10 s) et à la fin avec l'issue. La lecture les compare en passant ; au premier
écart, `Divergence` donne l'instant et un message (« Les règles du jeu ont changé depuis
l'enregistrement : à partir de 3 min 20 s, ce rejeu ne montre plus la bataille telle qu'elle a été
livrée. »), affiché par la barre de rejeu et le journal. Le rejeu reste visible (c'est une bataille
plausible), mais le joueur sait qu'il ne voit plus la sienne.

Le **format de fichier** est versionné (`header.format`, `REPLAY_FORMAT` = 1) : un autre format est
refusé avec un message (« vient d'une version plus récente / trop ancienne du jeu ») et le menu le
montre grisé. Un fichier d'exemple (`tests/fixtures/replay_sample.json`) doit toujours se relire : un
changement de structure sans incrément du format casse le test `the_sample_file_still_reads`.

### Sauts dans la barre de temps : copies en mémoire

`BattleSim` est `Clone` : la lecture garde une copie de la simulation toutes les `keyframe_seconds`
(20 s ; au plus `max_keyframes` = 90, la période s'allonge au-delà). Un saut en arrière repart de la
copie la plus proche, un saut en avant simule. Une copie coûte 0,03 ms et de l'ordre de 0,3 à 1 Mo (la
grille de relief du champ en est l'essentiel : 241 × 161 hauteurs au palier épique), soit au plus
quelques dizaines de Mo pour une longue bataille ; un saut en arrière prend quelques millisecondes. La re-simulation depuis le début (0,1 à 0,5 s
pour une bataille entière) aurait suffi pour les petites batailles mais aurait figé l'image lors des
sauts sur les grandes batailles épiques ; la sérialisation de l'état n'est pas nécessaire.

### Nombres à virgule exacts

`serde_json` lit par défaut certains nombres à 17 chiffres à un ulp près : un rejeu relu divergeait
(`time: 180.799999999994` relu `180.79999999999401`). La fonctionnalité `float_roundtrip` est activée
pour tout l'espace de travail (`core/Cargo.toml`). Les données écrites à la main (décimales courtes)
se lisent à l'identique ; les sauvegardes de campagne se relisent désormais exactement. Tous les tests
(`cargo test --workspace`) passent inchangés.

### Stockage

`BattleSim.save_replay(dir, titre)` écrit `rejeu-<heure unix>[-n].json` dans le dossier utilisateur
(`user://replays`, `ReplaysMenu.replays_dir()`), à la fin de chaque bataille (écran de fin) ; seuls les
`keep_count` (20) plus récents sont gardés. Réglages : `data/rules/battle_replay.json`
(`battle_replay_rules.schema.json`) ; fichier : `data/schemas/battle_replay.schema.json`. Pas
d'enregistrement en banc d'essai, en capture, ni en mode sans affichage sauf dossier de test imposé.

### Interface

- Écran de fin : bouton « Revoir la bataille » ; le rejeu part de l'enregistrement en mémoire, dans la
  même scène (terrain déjà construit), le résultat étant déjà appliqué à la campagne ; « Quitter le
  rejeu » rend la main comme « Retour à la campagne ».
- Menu principal : « Rejeux » (`ReplaysMenu`) liste titre, armées, vainqueur, durée, date ; « Revoir »
  lance la scène de bataille avec `--replay=<fichier>`.
- Scène en rejeu : barre en haut (`BattleReplayBar` : lecture/pause, ×1 ×2 ×4 ×8, barre de temps,
  temps, avis de divergence), caméra libre inchangée, sélection et suivi de caméra permis ; ordres,
  retraite générale, ordres du chef, déploiement, discours et vitesses de combat cachés ; le pont
  refuse toute commande (`issue_command`, `deploy_unit`, `start_battle`, `set_ai`...).

## Conséquences

- Une nouvelle entrée du pont qui modifie la bataille doit passer par `BattleSim::drive` (ou `note`)
  pour être enregistrée ; sinon le rejeu divergera — et le dira.
- Après un saut en arrière, figurines, sang, traits fichés, herbe couchée sont reconstruits à neuf :
  les corps tombés avant l'instant visé ne sont pas redessinés (rendu seulement).
- Les empreintes ne portent pas l'état du décor (feu, murailles) ni des étendards : un changement de
  ces seules règles apparaît dès qu'il touche un régiment (effectif, moral, position), ou à l'issue.
