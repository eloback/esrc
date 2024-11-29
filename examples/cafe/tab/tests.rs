#![cfg(test)]

use super::*;

fn open_tab() {
    let table_number: u64 = 42;
    let waiter: String = "Derek".into();

    let command = TabCommand::Open {
        table_number,
        waiter: waiter.clone(),
    };
    let aggregate = Tab::default();

    let expected_event = TabEvent::Opened {
        table_number,
        waiter,
    };
    let actual_event = aggregate.process(command);

    assert!(actual_event.is_ok());
    assert_eq!(expected_event, actual_event.unwrap());
}

#[test]
fn order_with_unopened_tab() {
    let item = Item {
        menu_number: 1,
        description: "drink".into(),
        price: 5.50,
    };

    let command = TabCommand::PlaceOrder { items: vec![item] };
    let aggregate = Tab::default();

    let expected_error = TabError::NotOpen;
    let actual_error = aggregate.process(command);

    assert!(actual_error.is_err());
    assert_eq!(expected_error, actual_error.unwrap_err());
}

#[test]
fn order() {
    let table_number: u64 = 42;
    let waiter: String = "Derek".into();
    let item = Item {
        menu_number: 1,
        description: "drink".into(),
        price: 5.50,
    };

    let command = TabCommand::PlaceOrder {
        items: vec![item.clone()],
    };
    let aggregate = Tab::default().apply(&TabEvent::Opened {
        table_number,
        waiter,
    });

    let expected_event = TabEvent::Ordered { items: vec![item] };
    let actual_event = aggregate.process(command);

    assert!(actual_event.is_ok());
    assert_eq!(expected_event, actual_event.unwrap());
}

#[test]
fn serve_twice() {
    let table_number: u64 = 42;
    let waiter: String = "Derek".into();
    let item = Item {
        menu_number: 1,
        description: "drink".into(),
        price: 5.50,
    };

    let command = TabCommand::MarkServed {
        menu_numbers: vec![item.menu_number],
    };
    let aggregate = Tab::default()
        .apply(&TabEvent::Opened {
            table_number,
            waiter,
        })
        .apply(&TabEvent::Ordered {
            items: vec![item.clone()],
        })
        .apply(&TabEvent::Served {
            menu_numbers: vec![item.menu_number],
        });

    let expected_error = TabError::AlreadyServed;
    let actual_error = aggregate.process(command);

    assert!(actual_error.is_err());
    assert_eq!(expected_error, actual_error.unwrap_err());
}

#[test]
fn serve() {
    let table_number: u64 = 42;
    let waiter: String = "Derek".into();
    let item = Item {
        menu_number: 1,
        description: "drink".into(),
        price: 5.50,
    };

    let command = TabCommand::MarkServed {
        menu_numbers: vec![item.menu_number],
    };
    let aggregate = Tab::default()
        .apply(&TabEvent::Opened {
            table_number,
            waiter,
        })
        .apply(&TabEvent::Ordered {
            items: vec![item.clone()],
        });

    let expected_event = TabEvent::Served {
        menu_numbers: vec![item.menu_number],
    };
    let actual_event = aggregate.process(command);

    assert!(actual_event.is_ok());
    assert_eq!(expected_event, actual_event.unwrap());
}

#[test]
fn close_with_underpayment() {
    let table_number: u64 = 42;
    let waiter: String = "Derek".into();
    let item = Item {
        menu_number: 1,
        description: "drink".into(),
        price: 5.50,
    };
    let amount_paid = 5.00;

    let command = TabCommand::Close { amount_paid };
    let aggregate = Tab::default()
        .apply(&TabEvent::Opened {
            table_number,
            waiter,
        })
        .apply(&TabEvent::Ordered {
            items: vec![item.clone()],
        })
        .apply(&TabEvent::Served {
            menu_numbers: vec![item.menu_number],
        });

    let expected_error = TabError::Unpaid;
    let actual_error = aggregate.process(command);

    assert!(actual_error.is_err());
    assert_eq!(expected_error, actual_error.unwrap_err());
}

#[test]
fn close() {
    let table_number: u64 = 42;
    let waiter: String = "Derek".into();
    let item = Item {
        menu_number: 1,
        description: "drink".into(),
        price: 5.50,
    };
    let amount_paid = 6.00;

    let command = TabCommand::Close { amount_paid };
    let aggregate = Tab::default()
        .apply(&TabEvent::Opened {
            table_number,
            waiter,
        })
        .apply(&TabEvent::Ordered {
            items: vec![item.clone()],
        })
        .apply(&TabEvent::Served {
            menu_numbers: vec![item.menu_number],
        });

    let expected_event = TabEvent::Closed {
        amount_paid,
        order_value: item.price,
        tip_value: amount_paid - item.price,
    };
    let actual_event = aggregate.process(command);

    assert!(actual_event.is_ok());
    assert_eq!(expected_event, actual_event.unwrap());
}
