# Lot B6 — IA tactique et terrain de site

Plan : `docs/design/2026-09-24-rapprochement-total-war.md`, suivi `docs/wip/tw.md`. Branche du worktree B6
(`worktree-agent-a87ff8639d57202f0`), non fusionnée.

## État : terminé (non fusionné)
- [x] IA, position défensive (`core/crates/sim-battle/src/ai.rs`, `defensive_cover`, `Cover`) : un camp
  en posture défensive cherche, à ±110 m latéralement et de 90 m en arrière à 110 m en avant de sa
  ligne de déploiement, une haie, un fossé ou une clôture à peu près parallèle au front (≥ 30 m ; poids
  haie 1, fossé 0,8, clôture 0,45) non masquée par un village devant, ou le bord du village face à
  l'ennemi (16 m à l'intérieur). Score = poids × longueur (plafonnée à 120) − 0,25 × écart latéral
  − 0,3 × écart en profondeur ; seuil 15. Sinon : hauteur (`high_ground`), comme avant.
  - Tireurs : alignés 7 m derrière l'obstacle (couvert de `hedge_between` à 14 m), en ordre latéral ;
    ils rejoignent la couverture avant de s'arrêter pour tirer ; derrière haie/fossé/village, les
    cavaliers ne les font plus reculer (leur charge s'y brise).
  - Ligne de mêlée : 30 m derrière les tireurs (ordre défensif habituel), ou juste derrière l'obstacle
    (12 m) s'il n'y a pas de tireurs.
- [x] Cavalerie (`charge_breaks`, `detour`, `charge_or_detour`) : ne charge plus à travers une haie ou
  un fossé ni dans un village (assaut contre la cavalerie adverse, tireurs isolés, flancs exposés,
  régiments ébranlés) ; contourne par l'extrémité la plus proche de l'obstacle (35 m au-delà), sinon
  attend sur son aile.
- [x] Déterminisme : ordre d'index partout, aucune source d'aléa nouvelle.
- [x] Sans site, rien ne change : empreintes de deux batailles complètes relevées avant B6, identiques
  après (`battles_without_a_site_are_unchanged`).
- [x] Rythme B4 : démo 1337 contact à 70,6 s (inchangé, la ferme est trop loin du centre) ; test
  `demo_contact_stays_near_seventy_seconds` (55-95 s) ; bocage + village sur 8 graines : contact
  < 180 s (`bocage_battles_still_engage`).
- [x] Lisibilité : `Battlefield::site_parts_fr` / `site_label_fr` (« Terre gelée » pour un sol sec
  d'hiver, saison, village/ferme, haies, fossés, clôtures, mares, rivière et gués, côte ouest/est),
  `BattleSeason::label_fr` ; pont : clé `site_label` dans `get_terrain()` et `get_site_label()` ;
  dialogue d'avant-bataille (ligne « Site : … », même aperçu `BattleSim` que la météo) ; HUD de
  bataille (ligne compacte sous la météo, `BattleHud.set_site`).
- [x] Tests `core/crates/sim-battle/tests/b6.rs` (9 + sondes ignorées) ; smoke : vérifications ajoutées
  dans le bloc bataille existant (22 « smoke OK », aucune « SCRIPT ERROR »).
- [x] Captures `docs/img/b6/b6_avant_bocage.png` (IA sans B6 : archers anglais en rase campagne) et
  `b6_apres_bocage.png` (archers derrière la haie), `--terrain=bocage --village --shot-at=50
  --camera=530,520,260,0`.

## Choix consignés
- Portée latérale réduite de 260 à 110 m : au-delà, l'attaquant (qui avance droit devant tant que
  l'ennemi est « devant ») mettait jusqu'à 4 min à trouver un défenseur décalé.
- L'assaut de cavalerie B4 contourne au lieu d'abandonner : sinon, sur le bocage, le premier contact
  pouvait passer de 80 s à 240 s.
- Clôture : ni couvert ni charge brisée en B5 ; gardée comme appui de dernier recours (poids faible).

## Points ouverts
- Bocage graine 5 (démo en bocage + village) : contact à 152 s au lieu de 75 s (chasse des archers
  montés anglais par les chevaliers français après le repli sur la haie) — dans la fenêtre B4.
- L'attaquant n'infléchit pas sa marche vers un défenseur décalé (`advance` inchangé pour préserver
  les batailles sans site).
- Minicarte : n'affiche pas encore haies/village.
- Le contournement vise l'extrémité de l'obstacle bloquant ; un réseau de haies (bocage dense) peut
  faire attendre la cavalerie.
