# A6-L4 recherche (reserve + file)

FAIT : core (reserve plafonnee = research_progress quand inactif, file research_queue, ordres queue_research/dequeue_research, regles dans data/rules/economy.json), pont (get_research_queue, get_research_reserve, queue_position), UI (Maj+clic, case Mettre en file, libelle court de la barre, infobulle). Tests Rust + smoke Godot verts.
OUVERT : verification visuelle de la barre (largeur 190/100) ; libelle compact non rafraichi au redimensionnement avant le prochain refresh.
