use crate::schema;
use crate::schema::users;
use anyhow::bail;
use anyhow::Result;
use bitcoin::secp256k1::PublicKey;
use coordinator_commons::RegisterParams;
use diesel::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use time::OffsetDateTime;

#[derive(Insertable, Queryable, Identifiable, Debug, Clone, Serialize, Deserialize)]
#[diesel(primary_key(id))]
pub struct User {
    #[diesel(deserialize_as = i32)]
    pub id: Option<i32>,
    pub pubkey: String,
    pub email: String,
    pub nostr: String,
    pub timestamp: OffsetDateTime,
    pub fcm_token: String,
}

impl From<RegisterParams> for User {
    fn from(value: RegisterParams) -> Self {
        User {
            id: None,
            pubkey: value.pubkey.to_string(),
            email: value.email.unwrap_or("".to_owned()),
            nostr: value.nostr.unwrap_or("".to_owned()),
            timestamp: OffsetDateTime::now_utc(),
            fcm_token: "".to_owned(),
        }
    }
}

pub fn all(conn: &mut PgConnection) -> QueryResult<Vec<User>> {
    users::dsl::users.load(conn)
}
pub fn by_id(conn: &mut PgConnection, id: String) -> QueryResult<Option<User>> {
    let x = users::table
        .filter(users::pubkey.eq(id))
        .first(conn)
        .optional()?;

    Ok(x)
}

pub fn upsert_email(
    conn: &mut PgConnection,
    trader_id: PublicKey,
    email: String,
) -> QueryResult<User> {
    let user: User = diesel::insert_into(users::table)
        .values(User {
            id: None,
            pubkey: trader_id.to_string(),
            email: email.clone(),
            nostr: "".to_owned(),
            timestamp: OffsetDateTime::now_utc(),
            fcm_token: "".to_owned(),
        })
        .on_conflict(schema::users::pubkey)
        .do_update()
        .set(users::email.eq(&email))
        .get_result(conn)?;
    Ok(user)
}

pub fn upsert_fcm_token(
    conn: &mut PgConnection,
    trader_id: PublicKey,
    token: String,
) -> Result<()> {
    tracing::debug!(%trader_id, token, "Updating token for client.");
    // TODO(holzeis): We should store a last login or freshness timestamp of the token into the
    // database. The recommendation is to assume any token not being updated after 2 months to be
    // stale.
    let affected_rows = diesel::insert_into(users::table)
        .values(User {
            id: None,
            pubkey: trader_id.to_string(),
            email: "".to_owned(),
            nostr: "".to_owned(),
            timestamp: OffsetDateTime::now_utc(),
            fcm_token: token.clone(),
        })
        .on_conflict(schema::users::pubkey)
        .do_update()
        .set(users::fcm_token.eq(&token))
        .execute(conn)?;

    if affected_rows == 0 {
        bail!("Could not update FCM token for node ID {trader_id}.");
    } else {
        tracing::debug!(%trader_id, %affected_rows, "Updated FCM token in DB.");
    }
    Ok(())
}
