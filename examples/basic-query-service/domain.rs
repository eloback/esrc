use esrc::version::{DeserializeVersion, SerializeVersion};
use esrc::{Aggregate, Event};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, DeserializeVersion, SerializeVersion, Event)]
pub enum OrderEvent {
    OrderPlaced { item: String, quantity: u32 },
    OrderShipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderCommand {
    PlaceOrder { item: String, quantity: u32 },
    ShipOrder,
}

#[derive(Debug, thiserror::Error, Serialize, Deserialize)]
pub enum OrderError {
    #[error("order already placed")]
    AlreadyPlaced,
    #[error("no order to ship")]
    NoOrder,
    #[error("order already shipped")]
    AlreadyShipped,
}

#[derive(Debug, Default)]
pub struct OrderAggregate {
    pub item: Option<String>,
    pub quantity: u32,
    pub shipped: bool,
}

impl Aggregate for OrderAggregate {
    type Command = OrderCommand;
    type Event = OrderEvent;
    type Error = OrderError;

    fn process(&self, command: Self::Command) -> Result<Self::Event, Self::Error> {
        match command {
            OrderCommand::PlaceOrder { item, quantity } => {
                if self.item.is_some() {
                    Err(OrderError::AlreadyPlaced)
                } else {
                    Ok(OrderEvent::OrderPlaced { item, quantity })
                }
            },
            OrderCommand::ShipOrder => {
                if self.item.is_none() {
                    Err(OrderError::NoOrder)
                } else if self.shipped {
                    Err(OrderError::AlreadyShipped)
                } else {
                    Ok(OrderEvent::OrderShipped)
                }
            },
        }
    }

    fn apply(mut self, event: &Self::Event) -> Self {
        match event {
            OrderEvent::OrderPlaced { item, quantity } => {
                self.item = Some(item.clone());
                self.quantity = *quantity;
            },
            OrderEvent::OrderShipped => {
                self.shipped = true;
            },
        }

        self
    }
}
