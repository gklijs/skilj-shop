---
name: skilj-shop-projections
description: Change Skilj Shop's seller fulfillment or finance projection fields, GraphQL visibility, capability gates, or seller isolation.
---

# Change a role-specific projection

1. Start in [src/shop.rs](../../../src/shop.rs). Edit the relevant state
   struct and its `project()` fold together. Both projections consume
   `OrderPlaced`, key state by order ID, and fold synchronously in the
   transaction that appends the event. Keep the fold free of I/O. If
   adding a projection, register it in [src/lib.rs](../../../src/lib.rs).
   When changing a state schema or fold, check whether existing stored
   projections need a rebuild. Registration alone does not replay events.
2. Keep both gates in view. `REQUIRES_CAPABILITY` controls which role
   can see a projection, including in its GraphQL schema. The `seller`
   owner tag composes with that gate to restrict scoped sellers to their
   own rows. Match the tag on `OrderPlaced`, and inspect the grants in
   [src/demo.rs](../../../src/demo.rs) for their read level, capability,
   and scope. If changing source fields, keep the capability-based
   redaction rules on `PlaceOrder` and `OrderPlaced` aligned.
   [Access model](../../../README.md#how-access-is-enforced)
3. Update the relevant [GraphQL examples](../../../queries/) and
   [walkthrough](../../../README.md#quickstart) if the visible fields
   change. Cover allowed reads, denied cross-role fields or projections,
   seller isolation in both directions, and role-filtered introspection
   in [tests/access_control.rs](../../../tests/access_control.rs).
   The test `roles_cannot_cross_payment_fulfillment_or_seller_boundaries`
   covers all of them. Run it with embedded PostgreSQL.

   ```sh
   cargo test --locked --test access_control roles_cannot_cross_payment_fulfillment_or_seller_boundaries
   ```

   If command validation changes, also run
   `cargo test --locked --test orders`. For setup and manual GraphQL checks,
   see [Develop and verify](../../../README.md#develop-and-verify) and
   [Try the access boundaries](../../../README.md#try-the-access-boundaries).

Projection mechanics adapted from the [skilj 0.4.0 projection guide](https://github.com/gklijs/SkilJ/blob/v0.4.0/skilj/agent-guides/skilj/references/projection.md)
under the [MIT license](LICENSE-MIT).
