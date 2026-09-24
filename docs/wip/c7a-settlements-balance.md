# C7a — repli du perdant, IA sur les colonies, équilibrage 50 tours

Spec : `docs/design/2026-09-24-echelle-colonies.md` (§ 4.3-4.5, § 7), suite de C4
(`docs/wip/c4-core-settlements.md`). Branche : `worktree-agent-a2e37f1c8b64c0bf2` (depuis `main` cda86a9).
Périmètre : `core/` et `data/` uniquement.

## État

- [x] Repli du perdant : règle C7a (`movement::retreat_target`), réglages `data/settlements/rules.json`
  § `retreat` (+ schéma), tests `sim-campaign/tests/c7a_retreat.rs`.
- [x] Sonde `core/crates/ai/examples/settlements_probe.rs` (50 tours × 8 graines, ~3,5 s par graine en release).
- [ ] IA sur les colonies (`ai_minimal`, `crates/ai`) : reprise des colonies perdues, garnisons
  frontalières, forteresses de niveau 4.
- [ ] Équilibrage (paramètres `data/`), mesures avant / après ci-dessous.

## Règle de repli (décision)

Ordre, déterministe (égalités départagées par l'id de colonie) :
1. colonie amie (à soi ou à un allié) sans armée ennemie, la plus proche sur le graphe, dans un rayon de
   `friendly_radius_steps` = 2 pas, par un chemin qui ne traverse aucune place ennemie ;
2. sinon colonie non tenue par un ennemi (neutre) sans armée ennemie dans un rayon de
   `neutral_radius_steps` = 1 pas ; les traînards coûtent `neutral_loss_percent` = 10 % ;
3. sinon **débandade** : `rout_loss_percent` = 50 % de pertes ; les survivants rejoignent la place amie la plus
   proche à toute distance (chemin sans place ennemie) si l'armée garde au moins
   `rout_dissolve_below_percent` = 30 % de ses effectifs, sinon elle se disperse (le général s'échappe).

Un attaquant battu revient toujours d'où il venait, sauf si une armée ennemie y est entre-temps.
Justification : une armée battue loin de ses places et cernée se dispersait (fuite de l'ost de Philippe VI
après Crécy, débandade après Poitiers, compagnies dispersées) ; une armée proche de ses places s'y
réfugiait ; le passage en terre neutre (Empire, Bretagne neutre) était courant mais coûtait des traînards.

## Mesures

Sonde : `cargo run --release -p ai --example settlements_probe -- 50 1 2 3 4 5 6 7 8`.

(à compléter)

## Prochaine étape

Mesure « avant » (main cda86a9 sans le repli), puis audit de l'IA sur les colonies.
