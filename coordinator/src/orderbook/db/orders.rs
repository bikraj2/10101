use crate::db::positions::ContractSymbol;
use crate::orderbook::db::custom_types::Direction;
use crate::orderbook::db::custom_types::MatchState;
use crate::orderbook::db::custom_types::OrderReason;
use crate::orderbook::db::custom_types::OrderState;
use crate::orderbook::db::custom_types::OrderType;
use crate::schema::matches;
use crate::schema::orders;
use bitcoin::secp256k1::PublicKey;
use diesel::prelude::*;
use diesel::result::QueryResult;
use diesel::PgConnection;
use orderbook_commons::NewOrder as OrderbookNewOrder;
use orderbook_commons::Order as OrderbookOrder;
use orderbook_commons::OrderReason as OrderBookOrderReason;
use orderbook_commons::OrderState as OrderBookOrderState;
use orderbook_commons::OrderType as OrderBookOrderType;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use time::OffsetDateTime;
use trade::Direction as OrderbookDirection;
use uuid::Uuid;

impl From<trade::Direction> for Direction {
    fn from(value: trade::Direction) -> Self {
        match value {
            trade::Direction::Long => Direction::Long,
            trade::Direction::Short => Direction::Short,
        }
    }
}

impl From<Direction> for trade::Direction {
    fn from(value: Direction) -> Self {
        match value {
            Direction::Long => trade::Direction::Long,
            Direction::Short => trade::Direction::Short,
        }
    }
}

impl From<OrderType> for OrderBookOrderType {
    fn from(value: OrderType) -> Self {
        match value {
            OrderType::Market => OrderBookOrderType::Market,
            OrderType::Limit => OrderBookOrderType::Limit,
        }
    }
}

impl From<OrderBookOrderType> for OrderType {
    fn from(value: OrderBookOrderType) -> Self {
        match value {
            OrderBookOrderType::Market => OrderType::Market,
            OrderBookOrderType::Limit => OrderType::Limit,
        }
    }
}

impl From<OrderState> for OrderBookOrderState {
    fn from(value: OrderState) -> Self {
        match value {
            OrderState::Open => OrderBookOrderState::Open,
            OrderState::Matched => OrderBookOrderState::Matched,
            OrderState::Taken => OrderBookOrderState::Taken,
            OrderState::Failed => OrderBookOrderState::Failed,
        }
    }
}

impl From<OrderBookOrderState> for OrderState {
    fn from(value: OrderBookOrderState) -> Self {
        match value {
            OrderBookOrderState::Open => OrderState::Open,
            OrderBookOrderState::Matched => OrderState::Matched,
            OrderBookOrderState::Taken => OrderState::Taken,
            OrderBookOrderState::Failed => OrderState::Failed,
        }
    }
}

#[derive(Queryable, Debug, Clone)]
struct Order {
    // this id is only internally but needs to be here or diesel complains
    #[allow(dead_code)]
    pub id: i32,
    pub trader_order_id: Uuid,
    pub price: f32,
    pub trader_id: String,
    pub direction: Direction,
    pub quantity: f32,
    pub timestamp: OffsetDateTime,
    pub order_type: OrderType,
    pub expiry: OffsetDateTime,
    pub order_state: OrderState,
    pub contract_symbol: ContractSymbol,
    pub leverage: f32,
    pub order_reason: OrderReason,
    pub stable: bool,
}

impl From<Order> for OrderbookOrder {
    fn from(value: Order) -> Self {
        OrderbookOrder {
            id: value.trader_order_id,
            price: Decimal::from_f32(value.price).expect("To be able to convert f32 to decimal"),
            trader_id: value.trader_id.parse().expect("to have a valid pubkey"),
            leverage: value.leverage,
            contract_symbol: value.contract_symbol.into(),
            direction: value.direction.into(),
            quantity: Decimal::from_f32(value.quantity)
                .expect("To be able to convert f32 to decimal"),
            order_type: value.order_type.into(),
            timestamp: value.timestamp,
            expiry: value.expiry,
            order_state: value.order_state.into(),
            order_reason: value.order_reason.into(),
            stable: value.stable,
        }
    }
}

impl From<OrderReason> for OrderBookOrderReason {
    fn from(value: OrderReason) -> Self {
        match value {
            OrderReason::Manual => OrderBookOrderReason::Manual,
            OrderReason::Expired => OrderBookOrderReason::Expired,
        }
    }
}

impl From<OrderBookOrderReason> for OrderReason {
    fn from(value: OrderBookOrderReason) -> Self {
        match value {
            OrderBookOrderReason::Manual => OrderReason::Manual,
            OrderBookOrderReason::Expired => OrderReason::Expired,
        }
    }
}

#[derive(Insertable, Debug, PartialEq)]
#[diesel(table_name = orders)]
struct NewOrder {
    pub trader_order_id: Uuid,
    pub price: f32,
    pub trader_id: String,
    pub direction: Direction,
    pub quantity: f32,
    pub order_type: OrderType,
    pub expiry: OffsetDateTime,
    pub order_reason: OrderReason,
    pub contract_symbol: ContractSymbol,
    pub leverage: f32,
    pub stable: bool,
}

impl From<OrderbookNewOrder> for NewOrder {
    fn from(value: OrderbookNewOrder) -> Self {
        NewOrder {
            trader_order_id: value.id,
            price: value
                .price
                .round_dp(2)
                .to_f32()
                .expect("To be able to convert decimal to f32"),
            trader_id: value.trader_id.to_string(),
            direction: value.direction.into(),
            quantity: value
                .quantity
                .round_dp(2)
                .to_f32()
                .expect("To be able to convert decimal to f32"),
            order_type: value.order_type.into(),
            expiry: value.expiry,
            order_reason: OrderReason::Manual,
            contract_symbol: value.contract_symbol.into(),
            leverage: value.leverage,
            stable: value.stable,
        }
    }
}

pub fn all(
    conn: &mut PgConnection,
    show_expired: bool,
    show_failed: bool,
) -> QueryResult<Vec<OrderbookOrder>> {
    let orders = match (show_expired, show_failed) {
        (true, true) => orders::dsl::orders.load::<Order>(conn)?,
        (false, false) => orders::table
            .filter(orders::expiry.gt(OffsetDateTime::now_utc()))
            .filter(orders::order_state.ne(OrderState::Failed))
            .load::<Order>(conn)?,
        (false, true) => orders::table
            .filter(orders::expiry.gt(OffsetDateTime::now_utc()))
            .load::<Order>(conn)?,
        (true, false) => orders::table
            .filter(orders::order_state.ne(OrderState::Failed))
            .load::<Order>(conn)?,
    };

    Ok(orders.into_iter().map(OrderbookOrder::from).collect())
}

/// Loads all orders by the given order direction and type
pub fn all_by_direction_and_type(
    conn: &mut PgConnection,
    direction: OrderbookDirection,
    order_type: OrderBookOrderType,
    filter_expired: bool,
) -> QueryResult<Vec<OrderbookOrder>> {
    let filters = orders::table
        .filter(orders::direction.eq(Direction::from(direction)))
        .filter(orders::order_type.eq(OrderType::from(order_type)))
        .filter(orders::order_state.eq(OrderState::Open));

    let orders: Vec<Order> = if filter_expired {
        filters
            .filter(orders::expiry.gt(OffsetDateTime::now_utc()))
            .load::<Order>(conn)?
    } else {
        filters.load::<Order>(conn)?
    };

    Ok(orders.into_iter().map(OrderbookOrder::from).collect())
}

pub fn get_all_orders(
    conn: &mut PgConnection,
    order_type: OrderBookOrderType,
    order_state: OrderBookOrderState,
    filter_expired: bool,
) -> QueryResult<Vec<OrderbookOrder>> {
    let filters = orders::table
        .filter(orders::order_state.eq(OrderState::from(order_state)))
        .filter(orders::order_type.eq(OrderType::from(order_type)));
    let orders: Vec<Order> = if filter_expired {
        filters
            .filter(orders::expiry.gt(OffsetDateTime::now_utc()))
            .load::<Order>(conn)?
    } else {
        filters.load::<Order>(conn)?
    };

    Ok(orders.into_iter().map(OrderbookOrder::from).collect())
}

/// Returns the number of affected rows: 1.
pub fn insert(
    conn: &mut PgConnection,
    order: OrderbookNewOrder,
    order_reason: OrderBookOrderReason,
) -> QueryResult<OrderbookOrder> {
    let new_order = NewOrder {
        order_reason: OrderReason::from(order_reason),
        ..NewOrder::from(order)
    };
    let order: Order = diesel::insert_into(orders::table)
        .values(new_order)
        .get_result(conn)?;

    Ok(OrderbookOrder::from(order))
}

/// Returns the number of affected rows: 1.
pub fn set_is_taken(
    conn: &mut PgConnection,
    id: Uuid,
    is_taken: bool,
) -> QueryResult<OrderbookOrder> {
    if is_taken {
        set_order_state(conn, id, orderbook_commons::OrderState::Taken)
    } else {
        set_order_state(conn, id, orderbook_commons::OrderState::Open)
    }
}

/// Returns the number of affected rows: 1.
pub fn set_order_state(
    conn: &mut PgConnection,
    id: Uuid,
    order_state: orderbook_commons::OrderState,
) -> QueryResult<OrderbookOrder> {
    let order: Order = diesel::update(orders::table)
        .filter(orders::trader_order_id.eq(id))
        .set((orders::order_state.eq(OrderState::from(order_state)),))
        .get_result(conn)?;

    Ok(OrderbookOrder::from(order))
}

pub fn set_expired_limit_orders_to_failed(
    conn: &mut PgConnection,
) -> QueryResult<Vec<OrderbookOrder>> {
    let expired_limit_orders: Vec<Order> = diesel::update(orders::table)
        .filter(orders::order_state.eq(OrderState::Open))
        .filter(orders::order_type.eq(OrderType::Limit))
        .filter(orders::expiry.lt(OffsetDateTime::now_utc()))
        .set(orders::order_state.eq(OrderState::Failed))
        .get_results(conn)?;

    Ok(expired_limit_orders
        .into_iter()
        .map(OrderbookOrder::from)
        .collect())
}

/// Returns the order by id
pub fn get_with_id(conn: &mut PgConnection, uid: Uuid) -> QueryResult<Option<OrderbookOrder>> {
    let x = orders::table
        .filter(orders::trader_order_id.eq(uid))
        .load::<Order>(conn)?;

    let option = x.get(0).map(|order| OrderbookOrder::from(order.clone()));
    Ok(option)
}

pub fn get_by_trader_id_and_state(
    conn: &mut PgConnection,
    trader_id: PublicKey,
    order_state: orderbook_commons::OrderState,
) -> QueryResult<Option<OrderbookOrder>> {
    orders::table
        .filter(orders::trader_id.eq(trader_id.to_string()))
        .filter(orders::order_state.eq(OrderState::from(order_state)))
        .first::<Order>(conn)
        .map(OrderbookOrder::from)
        .optional()
}

/// Get all the filled matches for all the limit orders generated by `trader_id`.
///
/// This can be used to calculate the implicit position of the maker, assuming that all the filled
/// matches were executed.
pub fn get_all_limit_order_filled_matches(
    conn: &mut PgConnection,
    trader_id: PublicKey,
) -> QueryResult<Vec<(Uuid, Decimal)>> {
    let orders = orders::table
        // We use `matches::match_order_id` so that we can verify that the corresponding app trader
        // order is in `match_state` _`Filled`_. The maker's match remains in `Pending` (since the
        // trade is not actually executed yet), which is not very informative.
        .inner_join(matches::table.on(matches::match_order_id.eq(orders::trader_order_id)))
        .filter(
            orders::trader_id
                .eq(trader_id.to_string())
                // Looking for `Matched`, `Limit` orders only, corresponding to the maker.
                .and(orders::order_type.eq(OrderType::Limit))
                .and(orders::order_state.eq(OrderState::Matched))
                // The corresponding app trader match is `Filled`.
                .and(matches::match_state.eq(MatchState::Filled)),
        )
        .select((
            // We use the order ID of the _match_ so that we get a unique order ID even if the same
            // limit order is partially filled more than once.
            matches::order_id,
            matches::quantity,
            orders::direction,
        ))
        .load::<(Uuid, f32, Direction)>(conn)?;

    let filled_matches = orders
        .into_iter()
        .map(|(order_id, quantity, direction_maker)| {
            let quantity = Decimal::from_f32(quantity).expect("to fit into Decimal");

            let quantity = match direction_maker {
                Direction::Long => quantity,
                Direction::Short => -quantity,
            };

            (order_id, quantity)
        })
        .collect();

    Ok(filled_matches)
}
