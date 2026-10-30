# Lot XP-R3a « Les cinq livres » (1/3)

Spec : `docs/audits/2026-10-29-experience.md` § R3. ADR 0818.

## État : TERMINÉ (lot 1 sur 3, en attente de fusion)
- [x] Cadre : `client/src/ui/kit/book.rs` (table `Books`, `book_bar`, `BOOK_WIDTH`), données `data/ui/books.fr.json` + schéma `ui_books`, `tab_bar_wrapped` (widgets).
- [x] Câblage : `client/src/campaign/panels/books.rs` (`OpenBook`, `plan` pur testé, touches, mémoire du dernier onglet, section de dump `books`) ; `OpenPanel::Hosted { panel, tab }` (+ `displaces`).
- [x] Livre Royaume (R) : panneau de faction (joueur), Économie, Féodalité partagent la rangée d'onglets et la largeur ; l'onglet « Commerce » du panneau de faction sort de la rangée du joueur (doublon avec Économie ▸ Commerce) ; « Ordres » (onglet Cour du panneau de faction) y reste en transition.
- [x] Livre Chronique (H, F5) : nouvel onglet Registre (`hud/audience.rs::registry_body`, extrait de la carte du Registre), Décisions, Lettres, Causes, Journal.
- [x] Alias : `hud:open_faction|economy|feudal` n'ont plus de bouton ; `click` les résout (`agent/mod.rs`) et P, M, Maj+F passent par eux. Anciens noms d'onglets conservés. Lanceur `hud:open_kingdom` (7 lanceurs au lieu de 9).
- [x] Touches : R Royaume (religion → Maj+N), H Chronique (garnison d'armée → Maj+H, F5 gardé).
- [x] Tests : unitaires (`book.rs`, `books.rs`), smoke `tests/smoke/books.txt`, snapshot de scénarios.

## Écarts / points ouverts
- Le journal et le fil de la carte ne sont pas retirés de l'écran de carte (la spec les rattache à R1/Audience).
- Le raccourci M (Économie) et la case du trésor (`hud:treasury`) ouvrent encore `OpenPanel::Economy` (bascule), sans passer par le livre.
- Alias `hud:open_*` : à retirer au prochain cycle ; mettre à jour `flow/integ_script.txt` et `tests/smoke/*.txt` à ce moment-là.

## Lots suivants
- XP-R3b : livres Armées (A : recrutement, Mes unités, rançons, ravitaillement) et Cour (C : personnages, Conseil, ordres de chevalerie, héritier ; retirer « Ordres » du Royaume).
- XP-R3c : Diplomatie (D, carte diplomatique) et intendant de province + « Chantiers du royaume ».
Pour ajouter un livre : une entrée dans `books.fr.json`, `Hosted` géré par le panneau hôte (comme `faction/mod.rs`), `book_bar` à la place de sa rangée d'onglets, un bras dans `books.rs` (`Hosted::current`, `close_message`).


# Lot XP-R3b « Les cinq livres » (2/3) : livres Armées, Cour, Diplomatie

## État : TERMINÉ côté code (cargo check + tests ciblés ; smoke et dumps laissés à la fusion)
- 5 livres en données (`data/ui/books.fr.json`, ADR 0818 amendé) : Royaume (+ Techniques, − Ordres), Armées (montre, recrutement, rançons), Cour (conseil, cour, actes, chevalerie, fiche, compétences, famille, décrets, ordres), Diplomatie (négociation, guerre, traités), Chronique.
- Hôtes câblés (`Hosted`/`Close`, `book_bar` à la place de leur rangée) : roster, recrutement, rançons, court, diplomatie, techniques. `OpenPanel::Close(panneau)` ajouté.
- Lanceurs : 7 -> 6 (5 livres + Codex) ; fenêtres distinctes : 22 -> 5 livres + 2 fiches (province avec siège, armée). Touches C, U, F3 lues par `books.rs`.
- Scripts mis à jour : `tests/smoke/{books,campaign,faction_*}.txt`, `flow/integ_script.txt`, `council/proof_script.txt`, `progress/target.rs` ; snapshot des scénarios régénéré.

## À faire à la fusion (une seule fois, sur main)
- Smoke : `books`, `campaign`, `faction_works`, `faction_assembly`, `campaign_audience`, `campaign_attention`, `campaign_turns` (tutoriel/cibles) ; dump `books` : 5 livres, `open` sur chaque étape de `books.txt`.
- Clippy complet, tests client complets.

## Points ouverts
- Intendant de province (file de chantiers par politique) : demande une règle du cœur ; « Chantiers du royaume » existe déjà (onglet Chantiers). -> XP-R3c.
- Plus de bouton de rançons (alias seulement) : pas de pastille d'alerte quand un captif existe ; à brancher sur l'Audience/alertes.
- Largeurs des hôtes Armées (roster 360, recrutement 620) non unifiées avec BOOK_WIDTH.
- Alias `hud:open_*` : à retirer au cycle suivant.
- Ravitaillement : jamais eu de fenêtre ; reste dans la montre.
