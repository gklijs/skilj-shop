# Skilj Shop

Skilj Shop is a small GraphQL demo built on **skilj 0.4.0**. It shows how
a seller can fulfill a purchase using its shipping details while finance
reconciles the same purchase using its payment details. The server
enforces what each role can see.

![Skilj Shop storefront showing the mechanical keyboard, USB-C dock, and felt desk mat](docs/storefront.png)

## Quickstart

### Launch the storefront

Install Rust and Cargo 1.95 or newer and a browser. On Debian 12 or
Ubuntu 26.04, install the build tools and the `curl` and `jq` tools used
in the walkthrough. [Install Rust and Cargo with rustup](https://rustup.rs/).

```sh
sudo apt-get update
sudo apt-get install -y build-essential cmake pkg-config libssl-dev ca-certificates git curl jq
```

On Debian 12 (bookworm) x86-64, install the runtime libraries and UTC
time zone data for embedded PostgreSQL.

```sh
sudo apt-get install -y libssl3 libxml2 liblz4-1 libzstd1 libgssapi-krb5-2 zlib1g tzdata
```

Clone the repo and start the demo as a regular user. The first launch
needs outbound HTTPS for the Rust dependencies and embedded PostgreSQL
download. On Ubuntu 26.04 or another host missing the embedded
binary's libraries, clone the repo and use the
[Docker PostgreSQL fallback](#docker-postgresql-fallback) to launch it.

```sh
git clone https://github.com/gklijs/skilj-shop.git
cd skilj-shop
unset DATABASE_URL
cargo run --locked
```

When the server prints `Storefront is ready`, open
[http://127.0.0.1:8080/](http://127.0.0.1:8080/) in your browser. The
storefront shows the fictional catalog and links to the seller and
finance walkthrough. The same server hosts GraphQL at
`http://127.0.0.1:8080/graphql`. Set `PORT` before startup to use another
port and open the storefront URL printed by the server.

### Set the demo tokens

Keep the server running. In a second terminal at the repository root,
paste the four complete `export` lines from startup. They set
`GRAPHQL_URL`, `SELLER_TOKEN`, `OTHER_SELLER_TOKEN`, and `FINANCE_TOKEN`.
The tokens expire after eight hours. Paste fresh exports after each restart.

Define this shell function to send a checked-in query to the endpoint
with the selected bearer token. It uses `order-1001` unless you supply
another order ID.

```sh
gql() {
  jq -n --rawfile query "$2" --arg order "${3:-order-1001}" \
    '{query: $query, variables: {order: $order}}' |
    curl --silent --show-error "$GRAPHQL_URL" \
      -H "Authorization: Bearer $1" -H 'Content-Type: application/json' \
      --data-binary @- | jq
}
```

### Fulfill order-1001 as the seller

Run the seller query. Its `data.projection` contains the order ID,
product description, quantity, and shipping address. [Seller query](queries/seller.graphql)

```sh
gql "$SELLER_TOKEN" queries/seller.graphql
```

```json
{
  "data": {
    "projection": {
      "orderId": "order-1001",
      "description": "Mechanical keyboard, ISO layout",
      "quantity": 1,
      "address": "Demo Street 12, 1234 AB Amsterdam"
    }
  }
}
```

### Reconcile order-1001 as finance

Run the finance query. The same `orderId` identifies the purchase.
The projection gives finance the customer email and the amount
paid, `12995` cents (€129.95). [Finance query](queries/finance.graphql)

```sh
gql "$FINANCE_TOKEN" queries/finance.graphql
```

```json
{
  "data": {
    "projection": {
      "orderId": "order-1001",
      "email": "alice@example.test",
      "pricePaidCents": 12995,
      "currency": "EUR"
    }
  }
}
```

## Try the access boundaries

### Swap roles or ask for a private field

Run each query with the other role's token. Look for
`Unknown type "shop_FinanceOrder"` in the first response's `errors` array
and `Unknown type "shop_FulfillmentOrder"` in the second.

```sh
gql "$SELLER_TOKEN" queries/finance.graphql
gql "$FINANCE_TOKEN" queries/seller.graphql
```

Adding `email` or `pricePaidCents` to the seller query's
`shop_FulfillmentOrder` fragment also fails GraphQL validation.

### Try the other seller's order

The first request returns GraphQL errors because `order-1002` belongs to
the other seller. The second returns that seller's fulfillment view,
and the third returns finance's view of the same order.

```sh
gql "$SELLER_TOKEN" queries/seller.graphql order-1002
gql "$OTHER_SELLER_TOKEN" queries/seller.graphql order-1002
gql "$FINANCE_TOKEN" queries/finance.graphql order-1002
```

### Inspect the schema for each role

Filter the introspection response to the shop's order projection types.
The seller command prints `shop_FulfillmentOrder` and the finance command
prints `shop_FinanceOrder`. Refresh your GraphQL client's schema when
switching tokens. [Introspection query](queries/introspection.graphql)

```sh
gql "$SELLER_TOKEN" queries/introspection.graphql |
  jq -r '.data.__schema.types[].name | select(. == "shop_FulfillmentOrder" or . == "shop_FinanceOrder")'
gql "$FINANCE_TOKEN" queries/introspection.graphql |
  jq -r '.data.__schema.types[].name | select(. == "shop_FulfillmentOrder" or . == "shop_FinanceOrder")'
```

## How access is enforced

| Data | Seller | Finance |
| --- | --- | --- |
| Order reference | Own orders | All orders |
| Product description and quantity | Yes | No |
| Shipping address | Yes | No |
| Customer email | No | Yes |
| Actual price paid and currency | No | Yes |

PostgreSQL stores the role grants. Sellers have the `fulfillment`
capability and a seller scope, and finance has the `finance` capability.
Both grants are read-only. The trusted demo operator places the seeded
orders on startup.

`PlaceOrder` emits `OrderPlaced`. Two synchronous, keyed projections,
`FulfillmentOrder` and `FinanceOrder`, copy the fields needed by their
audiences and require the corresponding capability. Owner tags restrict
seller reads to `seller-a` or `seller-b`. The command's consistency query
prevents duplicate purchases for a globally unique order ID.

The source command and event also declare matching capability-based
private fields. Generic command and event reads require admin access.
Even an admin without a field's capability receives `null` for it. This
is read-time redaction. GraphQL projection fields use camelCase
(`orderId`, `pricePaidCents`), while command payload JSON uses the Rust
field names (`order_id`, `price_paid_cents`).

Authentication validates RS256 JWTs against a loopback JWKS server. The
demo seeds roles and prints tokens for local use. The seeded description
and shipping address keep the customer's email and payment details out
of the fulfillment view.

## Database setup and troubleshooting

### Embedded PostgreSQL

On first launch, skilj downloads PostgreSQL **18.6.0** from the Theseus
GitHub releases into `~/.theseus/postgresql`. Allow outbound HTTPS to
GitHub and its release downloads. Cargo downloads the locked Rust
dependencies separately during the build.

If startup reports `libxml2.so.2: cannot open shared object file` or
`initdb` says it cannot find the `postgres` program, check the downloaded
binary's dependencies.

```sh
ldd ~/.theseus/postgresql/18.6.0/bin/postgres
```

A `not found` entry identifies a missing runtime library. On Debian 12,
`libxml2` supplies `libxml2.so.2`. Install the runtime packages from the
[quickstart](#launch-the-storefront) and retry. If the host supplies only a
newer ABI, use the [Docker PostgreSQL fallback](#docker-postgresql-fallback).

Stop the app with Ctrl-C. Relaunch with `cargo run --locked` to reuse the
default embedded database. To reset it, stop the app and run this command
from the repository root.

```sh
rm -rf -- .demo/postgres .demo/postgres.pgpass
```

This deletes the default local PostgreSQL cluster, including its orders
and grants, and its generated password file. The next embedded launch
recreates them. Keep `.demo/keys.json` with any demo database you still
use. It holds the demo encryption key shared by both launch modes in the
checkout.

### Docker PostgreSQL fallback

With Docker running and `openssl` installed, start a PostgreSQL 18.6
container with a dedicated `shop_demo` database and a loopback-only port.
Run these commands in the terminal where you will start the Rust app.

```sh
SKILJ_SHOP_DB_PASSWORD=$(openssl rand -hex 16)
docker run -d --name skilj-shop-postgres \
  -p 127.0.0.1:55432:5432 \
  -e POSTGRES_DB=shop_demo -e POSTGRES_USER=shop_demo \
  -e POSTGRES_PASSWORD="$SKILJ_SHOP_DB_PASSWORD" postgres:18.6-alpine
docker exec skilj-shop-postgres pg_isready -h 127.0.0.1 -U shop_demo -d shop_demo
```

Repeat the `pg_isready` command until it reports `accepting connections`,
then launch from the repository root. The `DATABASE_URL` setting bypasses
the embedded PostgreSQL startup and download.

```sh
DATABASE_URL="postgres://shop_demo:$SKILJ_SHOP_DB_PASSWORD@127.0.0.1:55432/shop_demo" cargo run --locked
```

Stop the app with Ctrl-C and repeat the launch command in the same
terminal to reuse the running container. Use
`docker stop skilj-shop-postgres` to stop PostgreSQL and keep its data.
Use `docker start skilj-shop-postgres` and repeat the readiness check to
resume it. To reset this database, stop the app and run the following
commands.

```sh
docker stop skilj-shop-postgres
docker rm -v skilj-shop-postgres
unset SKILJ_SHOP_DB_PASSWORD
```

`docker rm -v` deletes the dedicated container and its anonymous database
volume, including both purchases and the demo grants.

### Check a restart

After restarting in either mode, paste the new exports and run all four
reads. Each response should return the requested `data.projection.orderId`
and the fields for that role.

```sh
gql "$SELLER_TOKEN" queries/seller.graphql order-1001
gql "$FINANCE_TOKEN" queries/finance.graphql order-1001
gql "$OTHER_SELLER_TOKEN" queries/seller.graphql order-1002
gql "$FINANCE_TOKEN" queries/finance.graphql order-1002
```

After shutdown, `git status --short` should show no generated demo files.
Git ignores `target/`, `.demo/`, and local environment files such as `.env`.

### Setup verification

The embedded setup, first download, and test suite were exercised on
Debian 12.15 x86-64 using `rust:1-bookworm` with Rust 1.99.1. The
`libxml2.so.2` startup error and the Docker fallback were exercised on
Ubuntu 26.04.1 LTS x86-64 on WSL2 with Rust 1.100.0. Both setups served the
seeded purchases under their authorized roles after an app restart.
[First-run verification in issue #1](https://github.com/gklijs/skilj-shop/issues/1)

## Develop and verify

| Path | Contents |
| --- | --- |
| [src/main.rs](src/main.rs) and [src/lib.rs](src/lib.rs) | Server entry point and shop registration |
| [src/demo.rs](src/demo.rs) | Demo identities, JWTs, and seeded purchases |
| [src/shop.rs](src/shop.rs) | Order domain, projections, and access policies |
| [src/storefront.rs](src/storefront.rs) and [storefront/](storefront/) | Public routes, HTML, CSS, and product images |
| [queries/](queries/) | Seller, finance, and schema introspection operations |
| [tests/orders.rs](tests/orders.rs) | Order validation tests |
| [tests/access_control.rs](tests/access_control.rs) | PostgreSQL-backed access-control and storefront checks |
| [AGENTS.md](AGENTS.md) | Contributor guidance |

Install `rustfmt` and `clippy` with `rustup component add rustfmt clippy`
if needed. Run these commands from the repository root.

```sh
cargo build --locked
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

The access-control test starts its own isolated embedded PostgreSQL
database in the system temporary directory, even when `DATABASE_URL` is
set. It needs a regular user, the
[embedded runtime libraries](#launch-the-storefront), and download access
on first run. To run it by itself, use:

```sh
cargo test --locked --test access_control
```
