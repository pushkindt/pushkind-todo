use diesel::prelude::*;
use pushkind_common::repository::errors::{RepositoryError, RepositoryResult};

use crate::{
    domain::user::{NewUser as DomainNewUser, UpdateUser as DomainUpdateUser, User as DomainUser},
    models::user::{NewUser as DbNewUser, UpdateUser as DbUpdateUser, User as DbUser},
    repository::{DieselRepository, UserListQuery, UserReader, UserWriter},
};

impl From<crate::domain::types::TypeConstraintError> for RepositoryError {
    fn from(err: crate::domain::types::TypeConstraintError) -> Self {
        RepositoryError::ValidationError(err.to_string())
    }
}

impl UserReader for DieselRepository {
    fn get_user_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<DomainUser>> {
        use crate::schema::users;

        let mut conn = self.conn()?;

        let user = users::table
            .filter(users::id.eq(id))
            .filter(users::hub_id.eq(hub_id))
            .select(DbUser::as_select())
            .first::<DbUser>(&mut conn)
            .optional()?;

        Ok(user.map(|u| u.try_into()).transpose()?)
    }

    fn get_user_by_email(&self, email: &str, hub_id: i32) -> RepositoryResult<Option<DomainUser>> {
        use crate::schema::users;

        let mut conn = self.conn()?;
        let email = email.trim().to_lowercase();

        let user = users::table
            .filter(users::email.eq(email))
            .filter(users::hub_id.eq(hub_id))
            .select(DbUser::as_select())
            .first::<DbUser>(&mut conn)
            .optional()?;

        Ok(user.map(|u| u.try_into()).transpose()?)
    }

    fn list_users(&self, query: UserListQuery) -> RepositoryResult<(usize, Vec<DomainUser>)> {
        use crate::schema::users;

        let mut conn = self.conn()?;

        let search_pattern = query.search.as_ref().and_then(|term| {
            let trimmed = term.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(format!("%{}%", trimmed))
            }
        });

        let query_builder = || {
            let mut items = users::table
                .filter(users::hub_id.eq(query.hub_id))
                .into_boxed::<diesel::sqlite::Sqlite>();

            if let Some(pattern) = search_pattern.as_deref() {
                items = items.filter(users::name.like(pattern).or(users::email.like(pattern)));
            }

            items
        };

        let total = query_builder().count().get_result::<i64>(&mut conn)? as usize;

        let mut items = query_builder();

        if let Some(pagination) = &query.pagination {
            let page = pagination.page.max(1);
            let offset = ((page - 1) * pagination.per_page) as i64;
            let limit = pagination.per_page as i64;
            items = items.offset(offset).limit(limit);
        }

        let db_users = items
            .order(users::name.asc())
            .select(DbUser::as_select())
            .load::<DbUser>(&mut conn)?;

        Ok((
            total,
            db_users
                .into_iter()
                .map(|u| u.try_into())
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }
}

impl UserWriter for DieselRepository {
    fn create_or_update_user(&self, new_user: &DomainNewUser) -> RepositoryResult<DomainUser> {
        use crate::schema::users;

        let mut conn = self.conn()?;

        let db_new_user: DbNewUser = new_user.into();

        let db_update_user: DbUpdateUser = new_user.into();

        let db_user = diesel::insert_into(users::table)
            .values(&db_new_user)
            .on_conflict((users::email, users::hub_id))
            .do_update()
            .set(&db_update_user)
            .get_result::<DbUser>(&mut conn)?;

        Ok(db_user.try_into()?)
    }

    fn update_user(
        &self,
        user_id: i32,
        hub_id: i32,
        updates: &DomainUpdateUser,
    ) -> RepositoryResult<DomainUser> {
        use crate::schema::users;

        let mut conn = self.conn()?;
        let db_updates = DbUpdateUser::from(updates);

        let target = users::table
            .filter(users::id.eq(user_id))
            .filter(users::hub_id.eq(hub_id));

        let updated = diesel::update(target)
            .set(&db_updates)
            .returning(DbUser::as_returning())
            .get_result::<DbUser>(&mut conn)?;

        Ok(updated.try_into()?)
    }

    fn delete_user(&self, user_id: i32, hub_id: i32) -> RepositoryResult<()> {
        use crate::schema::users;

        let mut conn = self.conn()?;

        let target = users::table
            .filter(users::id.eq(user_id))
            .filter(users::hub_id.eq(hub_id));

        let deleted = diesel::delete(target).execute(&mut conn)?;
        if deleted == 0 {
            return Err(RepositoryError::NotFound);
        }

        Ok(())
    }

    fn touch_visited_at(&self, user_id: i32, hub_id: i32) -> RepositoryResult<()> {
        use crate::schema::users;

        let mut conn = self.conn()?;

        let target = users::table
            .filter(users::id.eq(user_id))
            .filter(users::hub_id.eq(hub_id));

        let visited_at = chrono::Local::now().naive_utc();

        diesel::update(target)
            .set(users::visited_at.eq(visited_at))
            .execute(&mut conn)?;

        Ok(())
    }
}
