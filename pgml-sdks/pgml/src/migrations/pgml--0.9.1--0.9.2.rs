use crate::{queries, query_builder};
use sqlx::Executor;
use sqlx::PgPool;
use tracing::instrument;

#[instrument(skip(pool))]
pub async fn migrate(pool: PgPool, _: Vec<i64>) -> anyhow::Result<()> {
    let collection_names: Vec<String> = sqlx::query_scalar("SELECT name FROM pgml.collections")
        .fetch_all(&pool)
        .await?;
    for collection_name in collection_names {
        let table_name = format!("{}.pipelines", collection_name);
        let pipeline_names: Vec<String> =
            sqlx::query_scalar(&query_builder!("SELECT name FROM %s", table_name))
                .fetch_all(&pool)
                .await?;
        for pipeline_name in pipeline_names {
            let table_name = format!("{}.{}_embeddings", collection_name, pipeline_name);
            pool.execute(
                query_builder!(
                    queries::CREATE_INDEX_USING_HNSW,
                    "",
                    "hnsw_vector_index",
                    &table_name,
                    "embedding vector_cosine_ops"
                )
                .as_str(),
            )
            .await?;
        }
    }

    // Required to set the default value for a not null column being added, but we want to remove
    // it right after
    let mut transaction = pool.begin().await?;
    transaction.execute("ALTER TABLE pgml.collections ADD COLUMN IF NOT EXISTS sdk_version text NOT NULL DEFAULT '0.9.2'").await?;
    transaction
        .execute("ALTER TABLE pgml.collections ALTER COLUMN sdk_version DROP DEFAULT")
        .await?;
    transaction.commit().await?;
    Ok(())
}
