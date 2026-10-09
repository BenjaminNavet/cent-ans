# 0277 — Capitaines recrutables, blessures temporaires, XP élargie

Statut : accepté

## Contexte
Écarts #1, #2, #6, #7, #17, #18 de `docs/wip/wh/personnages.md` : le trait `trait_wounded` était permanent, on ne pouvait pas
recruter de chef de guerre, l'XP ne venait que des batailles et du gouvernement, aucune montée de niveau n'était annoncée et
la fiche cachait les compteurs de faits d'armes.

## Décision
- Blessures : `Trait.expires_in_turns` (données). `skills::grant_trait` ouvre un compte à rebours
  (`CharacterState::trait_expiry`) de `ceil(durée × 100 / (100 + WoundRecovery))` tours, `WoundRecovery` étant celui du
  personnage (traits, compétences) et des techniques de sa faction. `characters::resolve_characters` décompte et retire le
  trait (événement `Medicine`). Un trait temporaire posé sans compte à rebours (événement, ancienne sauvegarde) en reçoit un.
  `trait_wounded` : 4 saisons.
- Capitaines : `Order::HireCaptain { settlement }`, règles dans `data/rules/captains.json` (coût au niveau de prix courant,
  plafond `base_cap + prestige / prestige_per_slot`, commandement 1-3, délai entre deux recrutements). Le capitaine est un
  `CharacterState` généré (`captain: true`, nom tiré de `data/names` selon la culture de la faction, maison = province).
  IA : `captains::ai_hire_captain` quand une armée n'a pas de général et que personne n'est libre.
- XP : `siege_xp`, `raid_xp`, `treaty_xp`, `ransom_xp` dans `data/rules/dynasty.json`. Traité : les deux souverains ;
  rançon : le captif libéré. `grant_experience` annonce chaque palier franchi par un événement `LevelUp` (niveau = 1 + points
  de compétence gagnés, `skills::level_of`).
- Fiche : bloc « Faits d'armes » (`get_character.feats`), pastille de niveau, durée restante d'une blessure.

## Conséquences
- Sauvegardes anciennes compatibles (champs `#[serde(default)]`).
- Les capitaines vieillissent, meurent et gagnent de l'XP comme les autres personnages.
- Le niveau n'ouvre pas encore d'emplacements (suite, compétences) : à faire avec les compétences par rôle (lot `charsb`).
