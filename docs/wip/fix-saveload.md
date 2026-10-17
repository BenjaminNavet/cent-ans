# fix-saveload (lot 5, défaut 14 du playtest 17/10)
État : champ `client` du wrapper de sauvegarde (core game-api) + `menu/save_extras.rs` (journal,
lettres, féodal récent) + conseiller sans ouverture hors tour 0. ADR 0456.
À vérifier : tutoriel `postponed` après chargement (voir bas de note), smoke.

## Résultat
- Repro (script s2 : Nouvelle partie, Non, 3 Entrée, F6, F9) : avant correctif journal 45 lignes / Monde 199 / lettres 25 / féodal 3 -> 0 après chargement ; après correctif identiques.
- Conseiller : l'ouverture ne rejoue plus (tour > 0 ignoré).
- Tutoriel `postponed` : non reproduit (reste 0 après chargement, progression dans les réglages).
- Restant : journal remis à zéro à chaque retour de bataille avant le correctif ; maintenant conservé.
