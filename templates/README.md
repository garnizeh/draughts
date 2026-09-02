# Server-rendered partials

HTMX partials rendered by the API layer (§5.1, §10.2). The server owns the
board, the legal-move highlights, and the commentary pane; the client owns
nothing but transient square selection.

Two rules govern everything in this directory:

1. **The authoritative legal-move state always comes from the server** (§10.2).
   A partial never computes legality, and Alpine.js is permitted only for local
   selection state.

2. **The commentary pane must render correctly when commentary is `null` or came
   from the canned fallback** (§5.1). The circuit breaker makes fallback a
   routine steady state, not an exceptional one, and the UI must not treat it as
   an error.

Commentary is fetched independently of the move response (§10.3): the board
lands in the time MCTS takes, not the time the model takes.
