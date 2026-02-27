# ES Fake

A lightweight, fake Elasticsearch (v8.10.0+) implementation written in Rust. This project is designed for testing and development environments, offering basic compatibility with official Elasticsearch clients without the overhead of running full clusters or heavy containers.

## Current Compatibility

The server emulates Elasticsearch behavior on port 9200.

### Supported API Endpoints:
* **Information & Cluster**:
    * `GET /` - Returns the standard ES tagline and version info.
    * `GET /_cluster/health` - Returns a simulated `green` cluster status.
* **Index Management**:
    * `PUT /{index}` - Create an index with optional mapping.
    * `HEAD /{index}` - Check if an index exists.
    * `PUT /{index}/_mapping` - Update or extend existing index mappings.
    * `DELETE /{index}` - Delete an entire index and its data.
    * `POST /{index}/_refresh` - Simulated refresh operation (no-op for consistency).
* **Document CRUD**:
    * `POST /{index}/_doc` - Index a document with an auto-generated `_id`.
    * `PUT /{index}/_doc/{id}` - Index or update a document with a specific `_id`.
    * `POST /{index}/_update/{id}` - Partial document update (merges new fields into existing source).
    * `GET /{index}/_doc/{id}` - Retrieve a specific document by ID.
    * `DELETE /{index}/_doc/{id}` - Delete a document by ID.
* **Bulk Operations**:
    * `POST /_bulk` and `POST /{index}/_bulk` - Supports `index` actions in NDJSON format.
* **Search & Analytics**:
    * `POST /{index}/_search` - Support for Query DSL and Aggregations.
    * `GET /{index}/_search` - Alternative search entry point.
    * `POST/GET /{index}/_count` - Fast document counting based on query.

### Supported Query DSL & Features:
* `match_all` - Retrieve all documents.
* `term` - Exact field matching (includes automatic handling of `.keyword` suffixes).
* `bool` - Filter combinations using `must`, `should`, and `must_not`.
* **Aggregations**: Support for `terms` aggregation (bucket-based grouping).
* **Pagination**: Support for `from` (offset) and `size` (limit) parameters.
* **Sorting**: Support for the `sort` field (including `.keyword`) with `asc` and `desc` orders.

### Mapping & Response Format:
* **Types**: `integer`, `float`, `boolean`, `keyword`, `text`, `date`.
* **Dynamic Mapping**: Configurable `dynamic: true/false` at the index level.
* **Mapping Updates**: Support for merging new properties into existing indices.
* **Standardized Errors**: Nested error structures (e.g., `error.root_cause`) to match official client expectations.
* **Metadata**: Responses include standard ES fields like `_shards`, `took`, and `timed_out`.

## Security
* **Basic Authentication**: Supported for user `elastic`.
* **Conditional Auth**: Can operate with or without a password based on the `ELASTIC_PASSWORD` environment variable.
* **Auth-less Support**: Fully compatible with environments where credentials are not required.

## Technical Stack
* **Framework**: Axum.
* **Storage**: In-Memory (DashMap) - data is cleared upon server restart.
* **Design**: Clean architecture with strictly no external ORM/SQL libraries in the domain.

## Execution
1. Set the environment variable (optional): `export ELASTIC_PASSWORD=your_password`
2. Run the project: `cargo run`
3. The server will listen on `http://0.0.0.0:9200`.