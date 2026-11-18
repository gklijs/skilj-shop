---
name: skilj-shop-storefront
description: Add or edit Skilj Shop's public catalog products, including product cards, EUR list prices, and bundled product images.
---

# Edit the storefront catalog

1. Edit the product cards in [storefront/index.html](../../../storefront/index.html).
   Follow the existing card structure with a name, short description, EUR
   list price with two decimal places, and an image with accurate alt text
   and intrinsic dimensions.
2. For a new image, add the WebP file under [storefront/](../../../storefront/)
   and serve it from an `/assets/` route in
   [src/storefront.rs](../../../src/storefront.rs). The router bundles assets
   at compile time, so rebuild or restart the demo after editing them.
3. Use fictional public catalog data. Keep order IDs, addresses, email,
   actual amounts paid, and demo tokens out of public HTML and assets.
   [Seeded purchases](../../../src/demo.rs) are independent fixtures.
4. Update catalog and asset assertions in
   [tests/access_control.rs](../../../tests/access_control.rs) for the
   changes. The helper `assert_storefront_serves_catalog_without_private_data`
   checks HTTP responses and guards against private data leaks. Run
   `cargo test --locked --test access_control` with the
   [embedded PostgreSQL prerequisites](../../../README.md#develop-and-verify).

Follow the [storefront quickstart](../../../README.md#launch-the-storefront)
and confirm the changed card, its image, and the displayed price at the
printed storefront URL.
