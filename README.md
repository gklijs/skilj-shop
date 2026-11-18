# Skilj Shop

A small GraphQL talk demo built on **skilj 0.4.0**, pinned exactly in
`Cargo.toml`. A shop resells products fulfilled by independent sellers.
The same purchase has two different, server-enforced views:

| Data | Seller | Finance |
| --- | --- | --- |
| Order reference | Own orders | All orders |
| Product description and quantity | Yes | No |
| Shipping address | Yes | No |
| Customer email | No | Yes |
| Actual price paid and currency | No | Yes |

Finance is modeled as a `finance` capability on a role's shop grant;
sellers hold `fulfillment` plus a seller scope. These are database grants,
not client-supplied role headers. Neither role can create orders or manage
access. The trusted demo operator places orders during startup.

## Set up a fresh checkout

Install the build tools on Debian 12 or Ubuntu 26.04. The command also
installs `curl` and `jq` for the GraphQL walkthrough.

```sh
sudo apt-get update
sudo apt-get install -y build-essential cmake pkg-config libssl-dev ca-certificates git curl jq
```

Install Rust and Cargo 1.95 or newer. [Rustup installation instructions](https://rustup.rs/)

Clone the repository and build from the lockfile.

```sh
git clone https://github.com/gklijs/skilj-shop.git
cd skilj-shop
cargo build --locked
```

## Run with embedded PostgreSQL

On Debian 12 (bookworm) x86-64, install the embedded PostgreSQL runtime
libraries and the system UTC time zone data.

```sh
sudo apt-get install -y libssl3 libxml2 liblz4-1 libzstd1 libgssapi-krb5-2 zlib1g tzdata
unset DATABASE_URL
cargo run --locked
```

Run as a regular user from the repository root. On the first run, skilj
downloads PostgreSQL **18.6.0** from the Theseus GitHub releases into
`~/.theseus/postgresql`. Allow outbound HTTPS to GitHub and its release
downloads, and budget time for the download before presenting. Cargo
downloads the locked Rust dependencies separately during the build.

The server seeds two fictional purchases and prints the GraphQL endpoint
(default `http://127.0.0.1:8080/graphql`) and four export commands. The
endpoint serves queries, mutations and subscriptions. Use the terminal
walkthrough below or a GraphQL client. Set `PORT` before startup to change
the HTTP port.

Stop the server with Ctrl-C. Start it again with the same command to keep
the purchases. Embedded data is stored in `.demo/postgres` with its
generated password in `.demo/postgres.pgpass`. To discard that default
embedded database, stop the demo before running this command.

```sh
rm -rf -- .demo/postgres .demo/postgres.pgpass
```

This removes the local embedded cluster, including its orders and grants,
and its password file. The demo recreates them on the next embedded run.
Keep `.demo/keys.json` with any demo database you still use. It contains
the demo's encryption key and is shared by both launch modes in a checkout.

### Troubleshoot embedded startup

Look for `libxml2.so.2: cannot open shared object file` in the startup
error. The same loader problem can surface as an `initdb` error about the
`postgres` program. Use `ldd` to check the downloaded binaries' dependencies.

```sh
ldd ~/.theseus/postgresql/18.6.0/bin/postgres
```

A `not found` entry identifies a missing runtime library. On Debian 12,
`libxml2` supplies `libxml2.so.2`. Install the runtime packages above and
retry. On a host that supplies only a newer ABI, such as Ubuntu 26.04,
use the external database setup below.

## Run with a dedicated Docker PostgreSQL database

With Docker running and `openssl` installed, start a PostgreSQL 18.6
container. It creates a dedicated `shop_demo` database and binds the
database port to loopback. Run these commands in the terminal where you
will start the Rust app.

```sh
SKILJ_SHOP_DB_PASSWORD=$(openssl rand -hex 16)
docker run -d --name skilj-shop-postgres \
  -p 127.0.0.1:55432:5432 \
  -e POSTGRES_DB=shop_demo -e POSTGRES_USER=shop_demo \
  -e POSTGRES_PASSWORD="$SKILJ_SHOP_DB_PASSWORD" postgres:18.6-alpine
docker exec skilj-shop-postgres pg_isready -h 127.0.0.1 -U shop_demo -d shop_demo
```

Repeat the `pg_isready` command until it reports `accepting connections`,
then launch the demo from the repository root.

```sh
DATABASE_URL="postgres://shop_demo:$SKILJ_SHOP_DB_PASSWORD@127.0.0.1:55432/shop_demo" cargo run --locked
```

`DATABASE_URL` bypasses the embedded server and its binary download. Stop
the Rust app with Ctrl-C. To check persistence, rerun the launch command
in the same shell with the container still running. Copy the new token
exports.

To stop PostgreSQL and keep the database, run `docker stop skilj-shop-postgres`.
Resume it with `docker start skilj-shop-postgres` and repeat the readiness check.
To discard the database, stop the Rust app before removing the container.

```sh
docker stop skilj-shop-postgres
docker rm -v skilj-shop-postgres
unset SKILJ_SHOP_DB_PASSWORD
```

`docker rm -v` deletes this dedicated container and its anonymous database
volume, including both purchases and the demo grants.

## Talk walkthrough

In a second terminal at the repository root, paste the four exports the
server printed, then define this helper. Tokens last eight hours and the
server generates new ones on each restart.

```sh
gql() {
  jq -n --rawfile query "$2" --arg order "${3:-order-1001}" \
    '{query: $query, variables: {order: $order}}' |
    curl --silent --show-error "$GRAPHQL_URL" \
      -H "Authorization: Bearer $1" -H 'Content-Type: application/json' \
      --data-binary @- | jq
}
```

1. **Fulfill an order.** `gql "$SELLER_TOKEN" queries/seller.graphql` shows
   a mechanical keyboard, quantity and shipping address. No email or price.
2. **Reconcile the payment.** `gql "$FINANCE_TOKEN" queries/finance.graphql`
   shows `alice@example.test`, `12995` cents (€129.95), and `EUR`. No address,
   product description or quantity. The order reference connects the views.
3. **Try crossing the boundary.** Run the finance query with `SELLER_TOKEN`,
   and the seller query with `FINANCE_TOKEN`. Both fail. Adding `email` or
   `pricePaidCents` to the seller's fragment also fails GraphQL validation.
4. **Inspect the schema.** Run `queries/introspection.graphql` as each role.
   `shop_FinanceOrder` is absent for sellers; `shop_FulfillmentOrder` is
   absent for finance. Refresh your client's schema when changing tokens.
5. **Try another seller's purchase.**
   `gql "$SELLER_TOKEN" queries/seller.graphql order-1002` fails even with
   the order ID. `OTHER_SELLER_TOKEN` can read it; finance can reconcile it.

For a restart check, copy the fresh exports and run both queries for each
purchase with its authorized roles. Each response should contain its
`data.projection.orderId` and the fields for that role.

```sh
gql "$SELLER_TOKEN" queries/seller.graphql order-1001
gql "$FINANCE_TOKEN" queries/finance.graphql order-1001
gql "$OTHER_SELLER_TOKEN" queries/seller.graphql order-1002
gql "$FINANCE_TOKEN" queries/finance.graphql order-1002
```

After shutdown, run `git status --short` from the repository root. Git
ignores the build output (`target/`), demo database and keys (`.demo/`),
and local environment files such as `.env`.

GraphQL projection fields use camelCase (`orderId`, `pricePaidCents`).
Command payload JSON uses the Rust field names (`order_id`, `price_paid_cents`).

## How access is enforced

`PlaceOrder` emits `OrderPlaced`. Two synchronous, keyed projections copy
only the fields their audience needs: `FulfillmentOrder` and `FinanceOrder`.
Each requires its matching capability. Owner tags additionally restrict
seller reads to `seller-a` or `seller-b`. Order IDs are globally unique;
the command's consistency query prevents duplicate purchases for an ID.

The source command **and** source event declare matching capability-based
private fields. Generic command/event reads require admin access, and even
an admin without a field's capability receives `null` for it. Projection
gates alone would not protect these payloads. Private fields provide
read-time redaction; this demo does not demonstrate encryption at rest.

Authentication uses real RS256 JWT validation against a loopback JWKS
server. Role seeding and printed tokens are conveniences for a local talk,
not a production login flow. The sample address and description contain
no customer contact information; free-text fields in a real checkout
would need rules to preserve that separation.

## Develop and verify

Install the `rustfmt` and `clippy` components if your Rust toolchain is
missing them (`rustup component add rustfmt clippy`).

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

The tests cover checkout validation and a real Postgres/GraphQL access
matrix: allowed views, forbidden fields and projections, introspection,
seller isolation, authentication, blocked writes and capability escalation,
raw command/event redaction even at admin level, immediate capability
revocation, and the absence of REST routes. The integration test starts an
isolated embedded database in the system temporary directory. Run
`cargo test --locked --test access_control` as a regular user with the
embedded runtime libraries and first-run download access described above.
The test creates and starts its own database even if `DATABASE_URL` is set.
It fails if the embedded server cannot start.

`src/shop.rs` contains the domain and policies, `src/demo.rs` the local
identities and seed data, `src/main.rs` the server, `queries/` the talk
operations, and `tests/` the regression checks. Use standard `rustfmt`
formatting and descriptive Rust names. Leave Git commits and pushes to
the user.

### Verification for issue #1

- Debian 12.15 x86-64 in a fresh `rust:1-bookworm` container with Rust
  1.99.1. Ran `cargo build --locked`, `cargo run --locked` with an empty
  PostgreSQL download cache, and `cargo test --locked`.
- Ubuntu 26.04.1 LTS x86-64 on WSL2 with Rust 1.100.0. Reproduced the
  `libxml2.so.2` startup error and verified the `postgres:18.6-alpine`
  fallback using the Docker startup, readiness, and cleanup commands above.

Both setups served `order-1001` with the printed seller token. After an
app restart, the four walkthrough queries returned both purchases under
their authorized roles. After shutdown, `git status --short` showed no
generated files.
