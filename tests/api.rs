use axum::{
    Router,
    body::{Body, to_bytes},
    extract::Request,
    http::{Response, StatusCode},
};

use gitnote::{
    api,
    render::GithubAPiRenderer,
    state,
    storage::{init_db_from_env, migrate},
};
use tower::util::ServiceExt;

struct TestApp {
    router: Router,
}

impl TestApp {
    async fn new() -> Self {
        let db = init_db_from_env().await;

        migrate(&db, "sql/01-CREATE_TABLE.sql")
            .await
            .expect("初始化sql失败");

        let app = state::AppState::new(db, GithubAPiRenderer::default(), gitnote::REPO_PATH);

        let router = api::setup_route(app);

        Self { router }
    }

    pub async fn request(&self, req: Request<Body>) -> Response<Body> {
        self.router
            .clone()
            .oneshot(req)
            .await
            .expect("oneshot fail")
    }
}

impl TestApp {
    async fn git_repo_sync(&self, oid: &str, lines: usize, msg: &str) {
        use serde_json::json;

        let req = Request::post("/api/repo/update")
            .header("Content-Type", "application/json")
            .body(Body::new(
                json!({
                    "refname": "refs/tags/cmd/rebuild",
                    "before": "0000000000000000000000000000000000000000",
                    "after": oid
                })
                .to_string(),
            ))
            .expect("请求失败");

        let resp = self.request(req).await;
        assert_eq!(StatusCode::OK, resp.status(), "{}", msg);
        let data = to_bytes(resp.into_body(), usize::MAX)
            .await
            .expect("读取数据失败");

        let text = String::from_utf8(data.to_vec()).expect("读取数据失败");
        assert_eq!(text.lines().count(), lines, "{}", msg);
    }

    async fn article_list(&self, msg: &str) -> Vec<serde_json::Value> {
        let req = Request::get("/api/articles")
            .body(Body::empty())
            .expect("请求失败");
        let resp = self.request(req).await;
        assert_eq!(StatusCode::OK, resp.status(), "{}", msg);
        let data = to_bytes(resp.into_body(), usize::MAX)
            .await
            .expect("读取数据失败");
        let json: Vec<serde_json::Value> = serde_json::from_slice(&data).expect("反序列化失败");
        json
    }

    async fn article(&self, slug: &str, code: StatusCode, msg: &str) {
        let req = Request::get(format!("/api/articles/{}", slug))
            .body(Body::empty())
            .expect("请求失败");
        let resp = self.request(req).await;
        assert_eq!(resp.status(), code, "{}", msg);
    }

    async fn group_list(&self, msg: &str) -> Vec<serde_json::Value> {
        let req = Request::get("/api/groups")
            .body(Body::empty())
            .expect("请求失败");
        let resp = self.request(req).await;
        assert_eq!(StatusCode::OK, resp.status(), "{}", msg);
        let data = to_bytes(resp.into_body(), usize::MAX)
            .await
            .expect("读取数据失败");
        let json: Vec<serde_json::Value> = serde_json::from_slice(&data).expect("反序列化失败");
        json
    }

    async fn tags_list(&self, msg: &str) -> Vec<serde_json::Value> {
        let req = Request::get("/api/tags")
            .body(Body::empty())
            .expect("请求失败");
        let resp = self.request(req).await;
        assert_eq!(StatusCode::OK, resp.status(), "{}", msg);
        let data = to_bytes(resp.into_body(), usize::MAX)
            .await
            .expect("读取数据失败");
        let json: Vec<serde_json::Value> = serde_json::from_slice(&data).expect("反序列化失败");
        json
    }
}

#[tokio::test]
#[ignore = "API测试 依赖真实数据库"]
async fn test_api() {
    let app = TestApp::new().await;
    // 同步第一个hash
    {
        app.git_repo_sync(
            "1b931e64cd251b0a98d9216b96ba4c51e69c7797",
            1,
            "提交第一个文件",
        )
        .await;

        let data = app
            .article_list("第一个文件还没有组策略 未设置public 应为空")
            .await;
        assert_eq!(data.len(), 0);
        assert_eq!(app.group_list("没有组策略 应为空").await.len(), 0);
        assert_eq!(app.tags_list("应为空").await.len(), 0);
        app.article("markdown-test", StatusCode::NOT_FOUND, "获取未公开的文章")
            .await;
    }

    // 同步第二个hash
    {
        app.git_repo_sync(
            "4db775450dee399c328935eb03fd4fcc6c60e333",
            2,
            "提交文件于组策略",
        )
        .await;

        let data = app.article_list("此时应可以获取到一个文件").await;
        assert_eq!(data.len(), 1);
        assert_eq!(app.group_list("此时应可以获取到一个组策略").await.len(), 1);
        assert_eq!(
            app.article_list("应可以获取到这个唯一的文件对应的的tags")
                .await
                .len(),
            1
        );
        app.article("markdown-test", StatusCode::OK, "获取文章")
            .await;
    }
}
