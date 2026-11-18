use skilj::CommandType;
use skilj_core::shared::CommandDecision;
use skilj_shop::{
    demo::sample_orders,
    shop::{PlaceOrder, ShopEvent},
};

#[test]
fn checkout_accepts_an_order_and_rejects_duplicate_ids() {
    let order = sample_orders().remove(0);
    match PlaceOrder::decide(&order, &[]) {
        CommandDecision::Accepted { events } => {
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].event_type, "OrderPlaced");
            assert_eq!(events[0].payload, serde_json::json!(order));
        }
        other => panic!("expected acceptance: {other:?}"),
    }
    assert!(matches!(
        PlaceOrder::decide(&order, &[ShopEvent::OrderPlaced(order.clone())]),
        CommandDecision::Rejected { .. }
    ));
}

#[test]
fn checkout_refuses_invalid_payment_and_fulfillment_data() {
    for field in [
        "price",
        "quantity",
        "currency",
        "email",
        "address",
        "description",
        "order",
        "seller",
    ] {
        let mut order = sample_orders().remove(0);
        match field {
            "price" => order.price_paid_cents = 0,
            "quantity" => order.quantity = -1,
            "currency" => order.currency = "USD".into(),
            "email" => order.email = "invalid".into(),
            "address" => order.address.clear(),
            "description" => order.description.clear(),
            "order" => order.order_id.clear(),
            "seller" => order.seller_id.clear(),
            _ => unreachable!(),
        }
        assert!(
            matches!(
                PlaceOrder::decide(&order, &[]),
                CommandDecision::Rejected { .. }
            ),
            "accepted invalid {field}"
        );
    }
}
