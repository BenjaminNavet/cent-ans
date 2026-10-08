# 0186 — Chantier SC : une seule voie de code

## Contexte
Après trois semaines de lots (M1 à A6), le dépôt compte ~190 k lignes de Rust, ~175 k de GDScript (dont 50 k de
tests), ~108 k de Python et 500 notes de travail. L'audit SC (20 zones, octobre 2026) relève les mêmes motifs
partout : chaque lot a laissé un interrupteur `--no-*` et un repli GDScript à côté de la voie native, des
sondes et captures jouées une fois, des helpers recopiés par fichier (noms, distance, hash, chargement JSON),
des valeurs par défaut Rust qui doublent `data/`, des commentaires qui racontent l'historique des lots.

## Décision
- Une seule voie de code : la voie native ou par défaut devient obligatoire ; les replis, interrupteurs A/B et
  shims de compatibilité sont supprimés. Les comparaisons A/B passent par git (deux commits), pas par un flag.
- Les sondes, bancs one-shot, scripts de capture de look-dev et outils de génération déjà joués sont supprimés ;
  git garde l'historique. Un outil ou un test sans consommateur (CI, `launch.sh`, build, `smoke.gd`, données)
  n'entre pas dans le dépôt.
- Les helpers communs ont un seul lieu : `data-model` (`GameData::province_name`…, `util`), `DataFile` côté
  GDScript, helpers de tests partagés.
- Les données ont une seule source : `data/` ; le Rust ne recopie pas les valeurs par défaut.
- Les commentaires disent quoi et pourquoi ; les étiquettes de lot et les références narratives aux ADR sont
  retirées du code (les ADR restent).
- Les sauvegardes antérieures ne sont plus lues (une seule version de format).
- Mécaniques : le joueur garde l'issue « bataille » des rencontres et les quatre issues de prise (avec les
  ruines). Toute autre suppression de mécanique a son propre ADR.

## Conséquences
- Moins de lignes et de chemins à tester ; le code dit ce que fait le jeu, pas comment il y est arrivé.
- Certains assets ne sont plus régénérables depuis le dépôt (pipelines retirés) : on les reprend dans
  l'historique git si besoin.
- Suivi du chantier : `docs/wip/sc-simplification.md`.
