use std::{env, str, time::Duration};

use sqlx::postgres::PgPoolOptions;

/// 数据库连接池类型
pub type Db = sqlx::PgPool;

/// 从环境变量 `DATABASE_URL` 初始化数据库连接池
pub async fn init_db_from_env() -> Db {
    let conn_url = env::var("DATABASE_URL")
        .ok()
        .expect("环境变量: `DATABASE_URL`: NotPresent");
    new_db_poll(&conn_url).await.unwrap()
}

/// 根据连接 URL 创建新的数据库连接池
///
/// 连接池配置：
///
/// - 最大空闲时间 60 秒
/// - 最大生存时间 1500 秒（约 25 分钟）
/// - 最大连接数 10
/// - 获取连接超时 2 秒
/// - 获取前测试连接
/// - 最小连接数 2
async fn new_db_poll(conn_url: &str) -> Result<Db, sqlx::Error> {
    PgPoolOptions::new()
        .idle_timeout(Duration::from_secs(60))
        .max_lifetime(Duration::from_secs(1500))
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(2))
        .test_before_acquire(true)
        .min_connections(2)
        .connect(conn_url)
        .await
}

/// 执行 SQL 文件中的迁移语句
///
/// 将文件内容按 `;` 分割，每条 SQL 单独执行
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
