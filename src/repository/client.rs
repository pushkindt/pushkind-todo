//! Diesel repository implementation for client persistence.
use diesel::prelude::*;
use pushkind_common::repository::errors::RepositoryResult;

use crate::{
    domain::client::{Client as DomainClient, NewClient as DomainNewClient},
    domain::types::{ClientId, HubId},
    models::client::{Client as DbClient, NewClient as DbNewClient},
    repository::{ClientReader, ClientWriter, DieselRepository},
};

impl ClientReader for DieselRepository {
    /// Load a client record by id within the specified hub.
    fn get_client_by_id(
        &self,
        id: ClientId,
        hub_id: HubId,
    ) -> RepositoryResult<Option<DomainClient>> {
        use crate::schema::clients;

        let mut conn = self.conn()?;
        let id = i32::from(id);
        let hub_id = i32::from(hub_id);

        let client = clients::table
            .filter(clients::id.eq(id))
            .filter(clients::hub_id.eq(hub_id))
            .select(DbClient::as_select())
            .first::<DbClient>(&mut conn)
            .optional()?;

        Ok(client.map(|client| client.try_into()).transpose()?)
    }

    /// List clients for the given hub ordered by name.
    fn list_clients(&self, hub_id: HubId) -> RepositoryResult<Vec<DomainClient>> {
        use crate::schema::clients;

        let mut conn = self.conn()?;
        let hub_id = i32::from(hub_id);

        let db_clients = clients::table
            .filter(clients::hub_id.eq(hub_id))
            .order(clients::name.asc())
            .select(DbClient::as_select())
            .load::<DbClient>(&mut conn)?;

        Ok(db_clients
            .into_iter()
            .map(|client| client.try_into())
            .collect::<Result<Vec<_>, _>>()?)
    }
}

impl ClientWriter for DieselRepository {
    /// Create or update a client record keyed by public id within the hub.
    fn create_or_update_client(
        &self,
        new_client: &DomainNewClient,
    ) -> RepositoryResult<DomainClient> {
        use crate::schema::clients;

        let mut conn = self.conn()?;
        let db_new: DbNewClient = new_client.into();
        let hub_id = new_client.hub_id.get();

        let db_client = conn.transaction::<DbClient, diesel::result::Error, _>(|conn| {
            let existing = clients::table
                .filter(clients::hub_id.eq(hub_id))
                .filter(clients::public_id.eq(new_client.public_id.as_str()))
                .select(DbClient::as_select())
                .first::<DbClient>(conn)
                .optional()?;

            if let Some(existing) = existing {
                return diesel::update(clients::table.filter(clients::id.eq(existing.id)))
                    .set((
                        clients::name.eq(new_client.name.as_str()),
                        clients::updated_at.eq(chrono::Utc::now().naive_utc()),
                    ))
                    .returning(DbClient::as_returning())
                    .get_result::<DbClient>(conn);
            }

            diesel::insert_into(clients::table)
                .values(&db_new)
                .returning(DbClient::as_returning())
                .get_result::<DbClient>(conn)
        })?;

        Ok(db_client.try_into()?)
    }
}
