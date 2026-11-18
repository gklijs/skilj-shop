pub mod demo;
pub mod shop;
mod storefront;

pub type Error = Box<dyn std::error::Error + Send + Sync>;

pub fn register(builder: skilj::SkiljBuilder) -> skilj::SkiljBuilder {
    builder
        .bounded_context(shop::BOUNDED_CONTEXT)
        .event_type::<shop::OrderPlaced>()
        .command_type::<shop::PlaceOrder>()
        .projection::<shop::FulfillmentOrder>()
        .projection::<shop::FinanceOrder>()
}
