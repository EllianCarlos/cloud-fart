# cloud-fart

`cloud-fart` is a production-oriented Rust service for mapping microservice dependencies and communication paths.
It ingests topology payloads from multiple adapters (CI/CD, git providers, deployment tools), stores a canonical model in SQLite, and optionally projects the graph into Neo4j for graph-native exploration.

## What the service does

- Runs an HTTP API for ingestion, query, and synchronization.
- Normalizes incoming graphs (dedupe services/edges and drop self-edges).
- Persists canonical graph state in SQLite.
- Projects graph relationships to Neo4j (when configured).
- Supports adapter-based multi-source ingestion (file adapter included; others can be added behind the same trait).

## API

- `GET /health` - readiness check (includes SQLite query check).
- `POST /v1/ingest` - ingest one dependency graph payload.
- `POST /v1/sync` - collect and ingest from all enabled adapters.
- `GET /v1/graph` - fetch aggregated graph snapshot.
- `GET /v1/services/:service/dependencies` - fetch a service's outgoing dependencies.

### Example ingestion payload

```json
{
  "source": "github-actions",
  "services": [
    { "name": "api", "metadata": { "team": "platform" } },
    { "name": "worker", "metadata": { "team": "platform" } }
  ],
  "communications": [
    {
      "source": "api",
      "target": "worker",
      "protocol": "http",
      "metadata": { "path": "/jobs" }
    }
  ]
}
```

## Configuration

Load order:
1. `config/service.toml` (optional)
2. environment variables with `CLOUD_FART__` prefix (e.g. `CLOUD_FART__SQLITE__PATH`)

Default file adapter locations are configured in `config/service.toml`.
A sample adapter payload is included at `fixtures/adapters/github-actions/sample.json`.

## Local development (devenv + direnv)

```bash
direnv allow
```

Start local Neo4j:

```bash
docker compose up -d neo4j
```

Run service:

```bash
cargo run
```

Run adapter sync example:

```bash
curl -X POST http://localhost:8080/v1/sync
curl http://localhost:8080/v1/graph
```

## Testing

Run full local checks:

```bash
./scripts/test-local.sh
```

Includes:
- `cargo fmt --all --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all`

## Roadmap

- Add first-class adapters for GitHub Actions, GitLab CI, ArgoCD, and Kubernetes.
- Add PostgreSQL implementation behind `GraphRepository`.
- Add authn/authz and ingestion provenance/auditing.
