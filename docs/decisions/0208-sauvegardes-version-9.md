# 0208 — Sauvegardes : version 9, plus de compatibilité ascendante

Statut : accepté (lot SC CC6).

## Contexte
L'état de campagne portait des couches de compatibilité avec les anciennes
sauvegardes : structures `Pre*Save`, `QueuedRecruitRepr`,
`LEGACY_RECRUIT_TURN`, `serde(default)` ajoutés seulement pour relire des
fichiers antérieurs, et des tests « old_save ». Le jeu n'est pas publié en
version stable ; ces couches coûtaient du code à chaque évolution de l'état.

## Décision
- `STATE_VERSION` passe à 9 ; toutes les couches de compatibilité disparaissent.
- `load_json` refuse une version inférieure (`OlderSave`) ou différente
  (`VersionMismatch`) au lieu de tenter une migration.
- Godot affiche la raison du refus (`SimFacade.last_load_error`) dans le
  toast de chargement.
- Les `serde(default, skip_serializing_if …)` qui compactent la sérialisation
  restent : ils ne relèvent pas de la compatibilité.

## Conséquences
- Les sauvegardes antérieures à la version 9 ne se chargent plus (accepté).
- Toute rupture future du format incrémente `STATE_VERSION` sans migration,
  tant que le jeu n'a pas de version publique.
