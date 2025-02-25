use crate::config;
use crate::db;
use crate::db::maybe_get_open_orders;
use crate::event;
use crate::event::EventInternal;
use crate::trade::order::orderbook_client::OrderbookClient;
use crate::trade::order::FailureReason;
use crate::trade::order::Order;
use crate::trade::order::OrderState;
use crate::trade::position;
use crate::trade::position::handler::update_position_after_order_submitted;
use crate::trade::position::PositionState;
use anyhow::anyhow;
use anyhow::bail;
use anyhow::Context;
use anyhow::Result;
use reqwest::Url;
use time::Duration;
use time::OffsetDateTime;
use uuid::Uuid;

const ORDER_OUTDATED_AFTER: Duration = Duration::minutes(5);

pub async fn submit_order(order: Order) -> Result<Uuid> {
    let url = format!("http://{}", config::get_http_endpoint());
    let orderbook_client = OrderbookClient::new(Url::parse(&url)?);

    if let Err(e) = position::handler::get_position_matching_order(&order) {
        order_failed(Some(order.id), FailureReason::OrderNotAcceptable, e)?;
        bail!("Could not submit order because extending/reducing the position is not part of the MVP scope");
    }

    db::insert_order(order)?;

    if let Err(err) = orderbook_client.post_new_order(order.into()).await {
        let order_id = order.id.to_string();
        tracing::error!(order_id, "Failed to post new order. Error: {err:#}");
        update_order_state_in_db_and_ui(order.id, OrderState::Rejected)?;
        if let Err(e) = position::handler::set_position_state(PositionState::Open) {
            bail!("Could not reset position to open because of {e:#}");
        }
        bail!("Could not post order to orderbook");
    }

    update_order_state_in_db_and_ui(order.id, OrderState::Open)?;
    update_position_after_order_submitted(&order)?;

    Ok(order.id)
}

/// Update order to state [`OrderState::Filling`].
pub(crate) fn order_filling(order_id: Uuid, execution_price: f32) -> Result<()> {
    let state = OrderState::Filling { execution_price };

    if let Err(e) = update_order_state_in_db_and_ui(order_id, state) {
        let e_string = format!("{e:#}");
        match order_failed(Some(order_id), FailureReason::FailedToSetToFilling, e) {
            Ok(()) => {
                tracing::debug!(%order_id, "Set order to failed, after failing to set it to filling");
            }
            Err(e) => {
                tracing::error!(%order_id, "Failed to set order to failed, after failing to set it to filling: {e:#}");
            }
        };

        bail!("Failed to set order {order_id} to filling: {e_string}");
    }

    Ok(())
}

pub(crate) fn order_filled() -> Result<Order> {
    let order_being_filled = get_order_being_filled()?;

    // Default the execution price in case we don't know
    let execution_price = order_being_filled.execution_price().unwrap_or(0.0);

    let filled_order = update_order_state_in_db_and_ui(
        order_being_filled.id,
        OrderState::Filled { execution_price },
    )?;
    Ok(filled_order)
}

/// Update order state to failed
///
/// If the order_id is know we load the order by id and set it to failed.
/// If the order_id is not known we load the order that is currently in `Filling` state and set it
/// to failed.
pub(crate) fn order_failed(
    order_id: Option<Uuid>,
    reason: FailureReason,
    error: anyhow::Error,
) -> Result<()> {
    tracing::error!("Failed to execute trade for order {order_id:?}: {reason:?}: {error:#}");

    let order_id = match order_id {
        None => get_order_being_filled()?.id,
        Some(order_id) => order_id,
    };

    update_order_state_in_db_and_ui(order_id, OrderState::Failed { reason })?;

    if let Err(e) = position::handler::set_position_state(PositionState::Open) {
        bail!("Could not reset position to open because of {e:#}");
    }

    Ok(())
}

pub async fn get_orders_for_ui() -> Result<Vec<Order>> {
    db::get_orders_for_ui()
}

pub fn get_async_order() -> Result<Option<Order>> {
    db::get_async_order()
}

fn get_order_being_filled() -> Result<Order> {
    let order_being_filled = match db::maybe_get_order_in_filling() {
        Ok(Some(order_being_filled)) => order_being_filled,
        Ok(None) => {
            bail!("There is no order in state filling in the database");
        }
        Err(e) => {
            bail!("Error when loading order being filled from database: {e:#}");
        }
    };

    Ok(order_being_filled)
}

pub fn check_open_orders() -> Result<()> {
    let orders_being_filled = match maybe_get_open_orders() {
        Ok(orders_being_filled) => orders_being_filled,
        Err(e) => {
            bail!("Error when loading open orders from database: {e:#}");
        }
    };

    let now = OffsetDateTime::now_utc();

    for order_being_filled in orders_being_filled {
        if order_being_filled.creation_timestamp + ORDER_OUTDATED_AFTER < now {
            order_failed(
                Some(order_being_filled.id),
                FailureReason::TimedOut,
                anyhow!("Order was not matched within {ORDER_OUTDATED_AFTER:?}"),
            )?;
        }
    }

    Ok(())
}

fn update_order_state_in_db_and_ui(order_id: Uuid, state: OrderState) -> Result<Order> {
    let order = db::update_order_state(order_id, state)
        .with_context(|| format!("Failed to update order {order_id} with state {state:?}"))?;

    ui_update(order);

    Ok(order)
}

fn ui_update(order: Order) {
    event::publish(&EventInternal::OrderUpdateNotification(order));
}
