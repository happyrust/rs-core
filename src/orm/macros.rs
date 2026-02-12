#[cfg(feature = "sea-orm")]
#[macro_export]
macro_rules! impl_db_op_trait {
    () => {
        impl DbOpTrait for Model {
            fn gen_insert_many(
                &self,
                models: Vec<serde_json::Value>,
                backend: DatabaseBackend,
            ) -> String {
                let active_models = models
                    .into_iter()
                    .map(|value| {
                        let model: Self = serde_json::from_value(value)
                            .expect("gen_insert_many: invalid model JSON payload");
                        let active_model: ActiveModel = model.into();
                        active_model
                    })
                    .collect::<Vec<ActiveModel>>();
                let insert = Entity::insert_many(active_models);
                insert.build(backend).to_string()
            }

            fn gen_create_table(&self, backend: DatabaseBackend) -> String {
                let schema = Schema::new(backend);
                let mut stmt: sea_orm::sea_query::TableCreateStatement =
                    schema.create_table_from_entity(Entity);
                stmt.if_not_exists();
                let f = backend.build(&stmt);
                f.to_string()
            }
        }
    };
}
