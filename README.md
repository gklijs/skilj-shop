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

## Run

Install Rust **1.95+**, a C compiler, CMake, pkg-config and OpenSSL development
headers. On Debian/Ubuntu: `sudo apt install build-essential cmake pkg-config libssl-dev`.
Then run:

```sh
cargo run --locked
```

Without `DATABASE_URL`, skilj starts embedded Postgres **18.6.0**, downloading
the binaries on the first run. Data persists in `.demo/postgres`; startup
seeds two fictional purchases and tolerates already-existing order IDs.
Allow time for the first build/download before presenting.

The downloaded Postgres binary also needs its system runtime libraries,
including **`libxml2.so.2`** on Linux. Recent Ubuntu releases ship a different
libxml2 ABI; use compatible runtime libraries or an external Postgres via
`DATABASE_URL` if embedded startup reports a missing shared library.

The server listens at **http://127.0.0.1:8080/graphql**. It serves GraphQL
queries, mutations and subscriptions; no REST router is mounted, and no
REST tokens are created. There is no bundled browser IDE; use your GraphQL
client or the terminal walkthrough below. `PORT` overrides `8080`.

Startup prints three `export` commands. Copy them into another terminal to
set `SELLER_TOKEN`, `OTHER_SELLER_TOKEN` and `FINANCE_TOKEN`. Tokens last eight
hours and use a fresh signing key on each startup. Restarting invalidates
the previous tokens. The operator token is kept inside the process.

Alternatively, point `DATABASE_URL` at a **dedicated demo database**:

```sh
DATABASE_URL=postgres://user:password@localhost:5432/shop_demo cargo run --locked
```

Stop with Ctrl-C. To reset embedded data, stop the server and remove `.demo/`.
Do not remove its contents while the server is running. External databases
are not reset by removing this directory.

## Talk walkthrough

The examples require `curl` and `jq`. Define this helper in the terminal
where you copied the tokens:

```sh
gql() {
  jq -n --rawfile query "$2" --arg order "${3:-order-1001}" \
    '{query: $query, variables: {order: $order}}' |
    curl --silent --show-error http://127.0.0.1:8080/graphql \
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
isolated embedded database and fails if it cannot start; it never silently
skips authorization checks.

`src/shop.rs` contains the domain and policies, `src/demo.rs` the local
identities and seed data, `src/main.rs` the server, `queries/` the talk
operations, and `tests/` the regression checks. Use standard `rustfmt`
formatting and descriptive Rust names. Leave Git commits and pushes to
the user.
