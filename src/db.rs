use std::{env, str, time::Duration};

use sqlx::postgres::PgPoolOptions;

pub type Db = sqlx::PgPool;

pub async fn init_db_from_env() -> Db {
    let conn_url = env::var("DATABASE_URL")
        .ok()
        .expect("环境变量: `DATABASE_URL`: NotPresent");
    new_db_poll(&conn_url).await.unwrap()
}

async fn new_db_poll(conn_url: &str) -> Result<Db, sqlx::Error> {
    PgPoolOptions::new()
        .idle_timeout(std::time::Duration::from_secs(60)) // 连接最大空闲时间 1 分钟（超过后自动关闭）
        .max_lifetime(std::time::Duration::from_secs(1500)) // 连接最大生存时间（定期重建 30 分钟
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(2)) // 获取超时时间
        .test_before_acquire(true) // 确保获取时检查连接
        .min_connections(2) // 重要！设置为 0 避免强制保留可能失效的最小连接（默认为0）
        .connect(conn_url)
        .await
}

#[allow(unused)]
pub async fn migrate(db: &Db, file: &str) -> Result<(), sqlx::Error> {
    let content = std::fs::read_to_string(file)?;

    let sqls: Vec<&str> = content.split(";").collect();
    for sql in sqls {
        if sql.is_empty() {
            continue;
        }
        sqlx::query(sql).execute(db).await?;
    }
    Ok(())
}
