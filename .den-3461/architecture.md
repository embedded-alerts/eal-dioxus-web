# Dioxus candidate inbox boundary

`eal-dioxus-web` is a server-rendered candidate inbox and evidence review surface.

It consumes tenant-scoped API read models. It does not query PostgreSQL directly, accept arbitrary crawl URLs, open a process-local echo WebSocket, mutate candidate state, or send provider traffic. The query-string filters affect only the server-rendered view and cannot select a tenant.

External indexes may supply discovery candidates, but only locally fetched, policy-approved, content-addressed, model-versioned revisions can appear in this inbox.
