use crate::db::models::ChannelState;
use crate::db::models::ContractSymbol;
use crate::db::models::Direction;
use crate::db::models::FailureReason;
use crate::db::models::Flow;
use crate::db::models::HtlcStatus;
use crate::db::models::OrderReason;
use crate::db::models::OrderState;
use crate::db::models::OrderType;
use crate::db::models::PositionState;
use diesel::backend;
use diesel::deserialize;
use diesel::deserialize::FromSql;
use diesel::serialize;
use diesel::serialize::IsNull;
use diesel::serialize::Output;
use diesel::serialize::ToSql;
use diesel::sql_types::Text;
use diesel::sqlite::Sqlite;

impl ToSql<Text, Sqlite> for OrderType {
    fn to_sql(&self, out: &mut Output<Sqlite>) -> serialize::Result {
        let text = match *self {
            OrderType::Market => "market".to_string(),
            OrderType::Limit => "limit".to_string(),
        };
        out.set_value(text);
        Ok(IsNull::No)
    }
}

impl FromSql<Text, Sqlite> for OrderType {
    fn from_sql(bytes: backend::RawValue<Sqlite>) -> deserialize::Result<Self> {
        let string = <String as FromSql<Text, Sqlite>>::from_sql(bytes)?;

        return match string.as_str() {
            "market" => Ok(OrderType::Market),
            "limit" => Ok(OrderType::Limit),
            _ => Err("Unrecognized enum variant".into()),
        };
    }
}

impl ToSql<Text, Sqlite> for OrderReason {
    fn to_sql(&self, out: &mut Output<Sqlite>) -> serialize::Result {
        let text = match *self {
            OrderReason::Manual => "Manual".to_string(),
            OrderReason::Expired => "Expired".to_string(),
        };
        out.set_value(text);
        Ok(IsNull::No)
    }
}

impl FromSql<Text, Sqlite> for OrderReason {
    fn from_sql(bytes: backend::RawValue<Sqlite>) -> deserialize::Result<Self> {
        let string = <String as FromSql<Text, Sqlite>>::from_sql(bytes)?;

        return match string.as_str() {
            "Manual" => Ok(OrderReason::Manual),
            "Expired" => Ok(OrderReason::Expired),
            _ => Err("Unrecognized enum variant".into()),
        };
    }
}

impl ToSql<Text, Sqlite> for OrderState {
    fn to_sql(&self, out: &mut Output<Sqlite>) -> serialize::Result {
        let text = match *self {
            OrderState::Initial => "initial".to_string(),
            OrderState::Rejected => "rejected".to_string(),
            OrderState::Open => "open".to_string(),
            OrderState::Failed => "failed".to_string(),
            OrderState::Filled => "filled".to_string(),
            OrderState::Filling => "filling".to_string(),
        };
        out.set_value(text);
        Ok(IsNull::No)
    }
}

impl FromSql<Text, Sqlite> for OrderState {
    fn from_sql(bytes: backend::RawValue<Sqlite>) -> deserialize::Result<Self> {
        let string = <String as FromSql<Text, Sqlite>>::from_sql(bytes)?;

        return match string.as_str() {
            "initial" => Ok(OrderState::Initial),
            "rejected" => Ok(OrderState::Rejected),
            "open" => Ok(OrderState::Open),
            "failed" => Ok(OrderState::Failed),
            "filled" => Ok(OrderState::Filled),
            "filling" => Ok(OrderState::Filling),
            _ => Err("Unrecognized enum variant".into()),
        };
    }
}

impl ToSql<Text, Sqlite> for ContractSymbol {
    fn to_sql(&self, out: &mut Output<Sqlite>) -> serialize::Result {
        let text = match *self {
            ContractSymbol::BtcUsd => "BtcUsd",
        };
        out.set_value(text);
        Ok(IsNull::No)
    }
}

impl FromSql<Text, Sqlite> for ContractSymbol {
    fn from_sql(bytes: backend::RawValue<Sqlite>) -> deserialize::Result<Self> {
        let string = <String as FromSql<Text, Sqlite>>::from_sql(bytes)?;

        return match string.as_str() {
            "BtcUsd" => Ok(ContractSymbol::BtcUsd),
            _ => Err("Unrecognized enum variant".into()),
        };
    }
}

impl ToSql<Text, Sqlite> for Direction {
    fn to_sql(&self, out: &mut Output<Sqlite>) -> serialize::Result {
        let text = match *self {
            Direction::Long => "Long",
            Direction::Short => "Short",
        };
        out.set_value(text);
        Ok(IsNull::No)
    }
}

impl FromSql<Text, Sqlite> for Direction {
    fn from_sql(bytes: backend::RawValue<Sqlite>) -> deserialize::Result<Self> {
        let string = <String as FromSql<Text, Sqlite>>::from_sql(bytes)?;

        return match string.as_str() {
            "Long" => Ok(Direction::Long),
            "Short" => Ok(Direction::Short),
            _ => Err("Unrecognized enum variant".into()),
        };
    }
}

impl ToSql<Text, Sqlite> for FailureReason {
    fn to_sql(&self, out: &mut Output<Sqlite>) -> serialize::Result {
        let text = match *self {
            FailureReason::TradeRequest => "TradeRequest",
            FailureReason::TradeResponse => "TradeResponse",
            FailureReason::NodeAccess => "NodeAccess",
            FailureReason::NoUsableChannel => "NoUsableChannel",
            FailureReason::ProposeDlcChannel => "ProposeDlcChannel",
            FailureReason::FailedToSetToFilling => "FailedToSetToFilling",
            FailureReason::OrderNotAcceptable => "OrderNotAcceptable",
            FailureReason::TimedOut => "TimedOut",
        };
        out.set_value(text);
        Ok(IsNull::No)
    }
}

impl FromSql<Text, Sqlite> for FailureReason {
    fn from_sql(bytes: backend::RawValue<Sqlite>) -> deserialize::Result<Self> {
        let string = <String as FromSql<Text, Sqlite>>::from_sql(bytes)?;

        return match string.as_str() {
            "TradeRequest" => Ok(FailureReason::TradeRequest),
            "TradeResponse" => Ok(FailureReason::TradeResponse),
            "NodeAccess" => Ok(FailureReason::NodeAccess),
            "NoUsableChannel" => Ok(FailureReason::NoUsableChannel),
            "ProposeDlcChannel" => Ok(FailureReason::ProposeDlcChannel),
            "FailedToSetToFilling" => Ok(FailureReason::FailedToSetToFilling),
            "OrderNotAcceptable" => Ok(FailureReason::OrderNotAcceptable),
            "TimedOut" => Ok(FailureReason::TimedOut),
            _ => Err("Unrecognized enum variant".into()),
        };
    }
}

impl ToSql<Text, Sqlite> for PositionState {
    fn to_sql(&self, out: &mut Output<Sqlite>) -> serialize::Result {
        let text = match *self {
            PositionState::Open => "Open",
            PositionState::Closing => "Closing",
            PositionState::Rollover => "Rollover",
        };
        out.set_value(text);
        Ok(IsNull::No)
    }
}

impl FromSql<Text, Sqlite> for PositionState {
    fn from_sql(bytes: backend::RawValue<Sqlite>) -> deserialize::Result<Self> {
        let string = <String as FromSql<Text, Sqlite>>::from_sql(bytes)?;

        return match string.as_str() {
            "Open" => Ok(PositionState::Open),
            "Closing" => Ok(PositionState::Closing),
            "Rollover" => Ok(PositionState::Rollover),
            _ => Err("Unrecognized enum variant".into()),
        };
    }
}

impl ToSql<Text, Sqlite> for HtlcStatus {
    fn to_sql(&self, out: &mut Output<Sqlite>) -> serialize::Result {
        let text = match *self {
            HtlcStatus::Pending => "Pending",
            HtlcStatus::Succeeded => "Succeeded",
            HtlcStatus::Failed => "Failed",
        };
        out.set_value(text);
        Ok(IsNull::No)
    }
}

impl FromSql<Text, Sqlite> for HtlcStatus {
    fn from_sql(bytes: backend::RawValue<Sqlite>) -> deserialize::Result<Self> {
        let string = <String as FromSql<Text, Sqlite>>::from_sql(bytes)?;

        return match string.as_str() {
            "Pending" => Ok(HtlcStatus::Pending),
            "Succeeded" => Ok(HtlcStatus::Succeeded),
            "Failed" => Ok(HtlcStatus::Failed),
            _ => Err("Unrecognized enum variant".into()),
        };
    }
}

impl ToSql<Text, Sqlite> for Flow {
    fn to_sql(&self, out: &mut Output<Sqlite>) -> serialize::Result {
        let text = match *self {
            Flow::Inbound => "Inbound",
            Flow::Outbound => "Outbound",
        };
        out.set_value(text);
        Ok(IsNull::No)
    }
}

impl FromSql<Text, Sqlite> for Flow {
    fn from_sql(bytes: backend::RawValue<Sqlite>) -> deserialize::Result<Self> {
        let string = <String as FromSql<Text, Sqlite>>::from_sql(bytes)?;

        return match string.as_str() {
            "Inbound" => Ok(Flow::Inbound),
            "Outbound" => Ok(Flow::Outbound),
            _ => Err("Unrecognized enum variant".into()),
        };
    }
}

impl ToSql<Text, Sqlite> for ChannelState {
    fn to_sql(&self, out: &mut Output<Sqlite>) -> serialize::Result {
        let text = match *self {
            ChannelState::Open => "Open",
            ChannelState::OpenUnpaid => "OpenUnpaid",
            ChannelState::Announced => "Announced",
            ChannelState::Pending => "Pending",
            ChannelState::Closed => "Closed",
            ChannelState::ForceClosedRemote => "ForceClosedRemote",
            ChannelState::ForceClosedLocal => "ForceClosedLocal",
        };
        out.set_value(text);
        Ok(IsNull::No)
    }
}

impl FromSql<Text, Sqlite> for ChannelState {
    fn from_sql(bytes: backend::RawValue<Sqlite>) -> deserialize::Result<Self> {
        let string = <String as FromSql<Text, Sqlite>>::from_sql(bytes)?;

        return match string.as_str() {
            "Open" => Ok(ChannelState::Open),
            "OpenUnpaid" => Ok(ChannelState::OpenUnpaid),
            "Announced" => Ok(ChannelState::Announced),
            "Pending" => Ok(ChannelState::Pending),
            "Closed" => Ok(ChannelState::Closed),
            "ForceClosedRemote" => Ok(ChannelState::ForceClosedRemote),
            "ForceClosedLocal" => Ok(ChannelState::ForceClosedLocal),
            _ => Err("Unrecognized enum variant".into()),
        };
    }
}

#[cfg(test)]
pub mod tests {
    use crate::db::custom_types::tests::customstruct::id;
    use crate::db::models::ContractSymbol;
    use crate::db::models::Direction;
    use crate::db::models::OrderState;
    use crate::db::models::OrderType;
    use diesel::connection::SimpleConnection;
    use diesel::insert_into;
    use diesel::prelude::*;
    use diesel::Connection;
    use diesel::RunQueryDsl;
    use diesel::SqliteConnection;

    #[derive(Insertable, Queryable, Identifiable, Debug, PartialEq, Clone)]
    #[diesel(table_name = customstruct)]
    pub struct SampleStruct {
        pub id: String,
        pub order_type: OrderType,
        pub order_state: OrderState,
        pub contract_symbol: ContractSymbol,
        pub direction: Direction,
    }

    diesel::table! {
        customstruct (id) {
            id -> Text,
            order_type -> Text,
            order_state -> Text,
            contract_symbol -> Text,
            direction -> Text,
        }
    }

    #[test]
    fn roundtrip_for_custom_types() {
        let mut connection = SqliteConnection::establish(":memory:").unwrap();
        connection
            .batch_execute(
                r#"
        create table customstruct (
            id TEXT PRIMARY KEY NOT NULL,
            order_type TEXT NOT NULL,
            order_state TEXT NOT NULL,
            contract_symbol TEXT NOT NULL,
            direction TEXT NOT NULL
        )"#,
            )
            .unwrap();

        let sample_struct = SampleStruct {
            id: "1".to_string(),
            order_type: OrderType::Limit,
            order_state: OrderState::Filled,
            contract_symbol: ContractSymbol::BtcUsd,
            direction: Direction::Short,
        };
        let i = insert_into(crate::db::custom_types::tests::customstruct::dsl::customstruct)
            .values(sample_struct.clone())
            .execute(&mut connection)
            .unwrap();

        assert_eq!(i, 1);

        let vec = crate::db::custom_types::tests::customstruct::dsl::customstruct
            .filter(id.eq("1".to_string()))
            .load::<SampleStruct>(&mut connection)
            .unwrap();

        assert_eq!(vec.len(), 1);

        let loaded_struct = vec.get(0).unwrap();
        assert_eq!(loaded_struct, &sample_struct);
    }
}
