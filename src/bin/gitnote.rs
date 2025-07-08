// 使用 mumalloc 作为全局内存分配器
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main]
async fn main() {
    gitnote::run().await;
}
