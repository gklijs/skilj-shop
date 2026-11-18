//! One order, two deliberately narrow read models. Capabilities protect
//! projections; private fields also protect generic event/command reads.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use skilj::{CommandType, EventType, Projection};
use skilj_core::{
    event_store::Event,
    plugin::BoundedContextEvent,
    shared::{
        CommandDecision, EventSpec, PrivateField, PrivateFieldKind, QueryItemMapping, TagMapping,
    },
};

pub const BOUNDED_CONTEXT: &str = "shop";
pub const FULFILLMENT: &str = "fulfillment";
pub const FINANCE: &str = "finance";

/// Money uses integer cents, never floating point. Address and description
/// stay separate from contact/payment data, even in the source payload.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OrderPayload {
    pub order_id: String,
    pub seller_id: String,
    pub email: String,
    pub price_paid_cents: i64,
    pub currency: String,
    pub address: String,
    pub description: String,
    pub quantity: i32,
}

fn tags() -> Vec<TagMapping> {
    vec![
        TagMapping::new("order", "order_id"),
        TagMapping::new("seller", "seller_id"),
    ]
}

fn private_fields() -> Vec<PrivateField> {
    [
        ("email", FINANCE),
        ("price_paid_cents", FINANCE),
        ("currency", FINANCE),
        ("address", FULFILLMENT),
        ("description", FULFILLMENT),
        ("quantity", FULFILLMENT),
    ]
    .into_iter()
    .map(|(field, capability)| PrivateField {
        field: field.into(),
        kind: PrivateFieldKind::Capability,
        capability: Some(capability.into()),
        addressee_field: None,
    })
    .collect()
}

pub struct OrderPlaced;

impl EventType for OrderPlaced {
    type Payload = OrderPayload;
    const NAME: &'static str = "OrderPlaced";
    fn tag_mappings() -> Vec<TagMapping> {
        tags()
    }
    fn owner_tag_key() -> Option<&'static str> {
        Some("seller")
    }
    fn private_fields() -> Vec<PrivateField> {
        private_fields()
    }
}

pub enum ShopEvent {
    OrderPlaced(OrderPayload),
}

impl BoundedContextEvent for ShopEvent {
    fn try_from_event(event: &Event) -> Option<Result<Self, serde_json::Error>> {
        match event.event_type.name.as_str() {
            "OrderPlaced" => Some(serde_json::from_str(&event.payload).map(Self::OrderPlaced)),
            _ => None,
        }
    }
}

/// Only the trusted checkout/operator writes orders. Seller and finance
/// roles are read-only and cannot replace source data or widen access.
pub struct PlaceOrder;

impl CommandType for PlaceOrder {
    type Payload = OrderPayload;
    type Event = ShopEvent;
    const NAME: &'static str = "PlaceOrder";
    fn tag_mappings() -> Vec<TagMapping> {
        tags()
    }
    fn consistency_query() -> Vec<QueryItemMapping> {
        // Read/lock only this order's history. The separate seller tag
        // enforces owner scope without widening the consistency boundary.
        vec![QueryItemMapping::tags(vec![TagMapping::new(
            "order", "order_id",
        )])]
    }
    fn owner_tag_key() -> Option<&'static str> {
        Some("seller")
    }
    fn private_fields() -> Vec<PrivateField> {
        private_fields()
    }
    fn decide(payload: &Self::Payload, history: &[Self::Event]) -> CommandDecision {
        let invalid = if payload.order_id.trim().is_empty() || payload.seller_id.trim().is_empty() {
            Some("order_id and seller_id are required")
        } else if payload.price_paid_cents <= 0 || payload.quantity <= 0 {
            Some("price and quantity must be positive")
        } else if payload.currency != "EUR" {
            Some("this demo accepts EUR only")
        } else if !payload.email.contains('@')
            || payload.address.trim().is_empty()
            || payload.description.trim().is_empty()
        {
            Some("email, address and description are required")
        } else if history
            .iter()
            .any(|ShopEvent::OrderPlaced(order)| order.order_id == payload.order_id)
        {
            Some("order already exists")
        } else {
            None
        };
        if let Some(reason) = invalid {
            return CommandDecision::Rejected {
                reason: reason.into(),
                kind: "invalid_order".into(),
            };
        }
        CommandDecision::Accepted {
            events: vec![EventSpec {
                event_type: "OrderPlaced".into(),
                payload: serde_json::json!(payload),
            }],
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct FulfillmentState {
    pub order_id: String,
    pub description: String,
    pub quantity: i32,
    pub address: String,
}

pub struct FulfillmentOrder;

impl Projection for FulfillmentOrder {
    type State = FulfillmentState;
    type Event = ShopEvent;
    const NAME: &'static str = "FulfillmentOrder";
    const REQUIRES_CAPABILITY: Option<&'static str> = Some(FULFILLMENT);
    const OWNER_TAG_KEY: Option<&'static str> = Some("seller");
    fn consumed_event_types() -> Vec<&'static str> {
        vec!["OrderPlaced"]
    }
    fn sync() -> bool {
        true
    }
    fn keys(event: &Self::Event) -> Vec<String> {
        let ShopEvent::OrderPlaced(order) = event;
        vec![order.order_id.clone()]
    }
    fn project(state: &mut Self::State, event: &Self::Event, _: &str) {
        let ShopEvent::OrderPlaced(order) = event;
        *state = FulfillmentState {
            order_id: order.order_id.clone(),
            description: order.description.clone(),
            quantity: order.quantity,
            address: order.address.clone(),
        };
    }
}

#[derive(Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct FinanceState {
    pub order_id: String,
    pub email: String,
    pub price_paid_cents: i64,
    pub currency: String,
}

pub struct FinanceOrder;

impl Projection for FinanceOrder {
    type State = FinanceState;
    type Event = ShopEvent;
    const NAME: &'static str = "FinanceOrder";
    const REQUIRES_CAPABILITY: Option<&'static str> = Some(FINANCE);
    const OWNER_TAG_KEY: Option<&'static str> = Some("seller");
    fn consumed_event_types() -> Vec<&'static str> {
        vec!["OrderPlaced"]
    }
    fn sync() -> bool {
        true
    }
    fn keys(event: &Self::Event) -> Vec<String> {
        let ShopEvent::OrderPlaced(order) = event;
        vec![order.order_id.clone()]
    }
    fn project(state: &mut Self::State, event: &Self::Event, _: &str) {
        let ShopEvent::OrderPlaced(order) = event;
        *state = FinanceState {
            order_id: order.order_id.clone(),
            email: order.email.clone(),
            price_paid_cents: order.price_paid_cents,
            currency: order.currency.clone(),
        };
    }
}
