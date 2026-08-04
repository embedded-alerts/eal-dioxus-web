# eal-dioxus-web

Dioxus SSR + Axum + WebSocket comparison server for Embedded Alerts.

**Product:** Embedded Alerts — Embedding-based alerting for semantically relevant new information.

Define semantic alert rules, ingest source documents, compare embeddings, rank matches, and deliver explainable notifications.

## Safety and production boundary

Similarity scores are ranking signals, not truth guarantees. Production ingestion must respect source terms, robots rules, privacy requirements, retention limits, and notification consent.

This repository is an executable bootstrap, not a production deployment. Before live
use, add authentication, tenant authorization, rate limits, durable migrations,
observability, backups, incident response, dependency review, and secret management.


This comparison surface preserves the same health and WebSocket behavior as the other web servers while exploring Dioxus SSR and a future multi-platform path.
