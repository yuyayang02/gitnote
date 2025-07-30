use chrono::{Datelike, Duration, Local, NaiveTime};
use tracing::instrument;

use super::GitBareRepository;
use crate::error::Result;

/// Trait: 可转换为 NaiveTime，用于每日定时任务触发时间的统一表示。
///
/// 支持多种元组格式，方便灵活指定小时、分钟、秒数。
pub trait IntoNaiveTime {
    /// 转换为 chrono 的 NaiveTime 类型。
    fn to_time(&self) -> NaiveTime;
}

// 仅包含小时，默认分钟和秒为 0
impl IntoNaiveTime for (u32,) {
    fn to_time(&self) -> NaiveTime {
        NaiveTime::from_hms_opt(self.0, 0, 0).unwrap()
    }
}

// 包含小时和分钟，秒默认为 0
impl IntoNaiveTime for (u32, u32) {
    fn to_time(&self) -> NaiveTime {
        NaiveTime::from_hms_opt(self.0, self.1, 0).unwrap()
    }
}

// 包含小时、分钟和秒
impl IntoNaiveTime for (u32, u32, u32) {
    fn to_time(&self) -> NaiveTime {
        NaiveTime::from_hms_opt(self.0, self.1, self.2).unwrap()
    }
}

/// ArchiveTagger 是一个自动归档标签调度器，
/// 负责在每个季度结束日期自动创建对应的 Git 标签，
/// 标签格式为 `archive/YYYY-Qn`，其中 n 表示季度（1~4）。
///
/// 标签用于标记季度归档起点，便于后续的归档处理或界面展示。
///
/// # 设计要点
/// - 支持灵活指定调度任务执行时间（小时/分钟/秒）
/// - 以固定间隔（通常为 24 小时）循环执行检查
/// - 当天为季度末则自动创建标签，已存在标签会被覆盖
/// - 使用 bare 仓库封装对象执行操作
pub struct ArchiveTagger<T> {
    repo: GitBareRepository,
    run_at_time: T,
    interval: u64,
}

impl<T: IntoNaiveTime> ArchiveTagger<T> {
    /// 构造新的归档调度器
    ///
    /// - `repo`: GitBareRepository，裸仓库访问封装
    /// - `run_at_time`: 任务每日触发时间点
    /// - `interval`: 任务触发间隔秒数（通常为 86400，即每天一次）
    pub fn new(repo: GitBareRepository, run_at_time: T, interval: u64) -> Self {
        Self {
            repo,
            run_at_time,
            interval,
        }
    }

    /// 计算距离下一个触发时间的延迟 Duration，
    /// 该延迟用于首次调度时的等待。
    ///
    /// 计算规则：
    /// - 若当前时间早于当天设定的触发时间，则延迟为两者时间差
    /// - 否则延迟至第二天的设定时间
    fn compute_initial_delay(&self) -> tokio::time::Duration {
        let now = Local::now();

        let target_time = self.run_at_time.to_time();
        let now_time = now.time();

        let delay = if now_time < target_time {
            (target_time - now_time).to_std().unwrap()
        } else {
            let tomorrow_target = target_time + Duration::days(1);
            (tomorrow_target - now_time).to_std().unwrap()
        };

        tokio::time::Duration::from_secs(delay.as_secs())
    }

    /// 启动调度任务，周期性检查并根据日期创建季度归档标签。
    ///
    /// 任务流程：
    /// 1. 根据设定时间计算首次启动延迟
    /// 2. 每隔指定间隔执行一次检查
    /// 3. 若检测到当前为季度末，则创建对应标签（强制覆盖）
    ///
    /// 使用 Tokio 定时器进行异步调度，适合长期运行的后台任务。
    #[instrument(name = "archive tagger", skip_all)]
    pub async fn run_scheduled_task(&self) {
        let initial_delay = self.compute_initial_delay();
        tracing::info!(
            "Archive scheduler will start in {} seconds",
            initial_delay.as_secs()
        );

        let start_instant = tokio::time::Instant::now() + initial_delay;

        let mut ticker = tokio::time::interval_at(
            start_instant,
            tokio::time::Duration::from_secs(self.interval),
        );

        loop {
            ticker.tick().await;

            tracing::info!("Running scheduled archive tag check");

            if let Err(err) = self.maybe_tag_quarter_archive() {
                tracing::error!("Archive tag check failed: {:?}", err);
            } else {
                tracing::info!("Archive tag check completed successfully");
            }
        }
    }

    /// 格式化归档标签的完整 Git 引用名。
    ///
    /// 例如传入 "2025-Q2" 返回 "archive/2025-Q2"。
    fn tag_name(tag: &str) -> String {
        format!("archive/{tag}")
    }

    /// 在当前 HEAD 上创建或更新标签，标签名需包含完整路径。
    ///
    /// 若标签已存在，会警告并强制覆盖。
    fn push_tag(&self, tag_name: &str) -> Result<()> {
        let repo = self.repo.repo()?;

        if repo.refname_to_id(&tag_name).is_ok() {
            tracing::warn!("Tag '{tag_name}' already exists and will be overwritten");
        }

        let head = repo.head()?.peel_to_commit()?;
        let sig = repo.signature()?;

        repo.tag(
            &tag_name,
            head.as_object(),
            &sig,
            &format!("archive: auto tag {tag_name}"),
            true,
        )?;

        Ok(())
    }

    /// 直接创建当前季度归档标签。
    ///
    /// 计算当前季度标签名并调用 push_tag 完成创建。
    fn tag_quarter_archive(&self) -> Result<()> {
        let tag = current_quarter_as_tag();

        let tag_name = Self::tag_name(&tag);

        self.push_tag(&tag_name)?;

        tracing::info!("Created archive tag: {tag_name}");
        Ok(())
    }

    /// 检查当前日期是否为季度末，若是则创建归档标签。
    ///
    /// 若非季度末则跳过操作。
    fn maybe_tag_quarter_archive(&self) -> Result<()> {
        if !is_quarter_end() {
            tracing::debug!("Not end of quarter; skipping tag creation");
            return Ok(());
        }

        self.tag_quarter_archive()?;
        Ok(())
    }
}

/// 判断是否为季度末日：3/31、6/30、9/30、12/31。
///
/// 用于判断是否触发季度归档标签创建。
fn is_quarter_end() -> bool {
    let today = Local::now().date_naive();
    matches!(
        (today.month(), today.day()),
        (3, 31) | (6, 30) | (9, 30) | (12, 31)
    )
}

/// 获取当前日期对应的季度标签字符串，格式为 `YYYY-Qn`。
///
/// 示例：2025年5月返回 "2025-Q2"。
fn current_quarter_as_tag() -> String {
    let today = Local::now();
    let quarter = (today.month0() / 3 + 1) as u8;
    format!("{}-Q{quarter}", today.year())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Local};

    #[test]
    fn test_is_quarter_end_on_non_quarter_day() {
        // 这个测试不保证运行当天是非季度末，但调用不会panic
        // 更精准测试需要mock时间或重构代码
        let result = is_quarter_end();
        // 结果可能为true或false，打印以供参考
        println!("Is today quarter end? {}", result);
    }

    #[test]
    fn test_current_quarter_as_tag_format() {
        let tag = current_quarter_as_tag();
        let today = Local::now();

        // 应该包含当前年份和季度
        let expected_quarter = (today.month0() / 3 + 1) as u8;
        let expected_prefix = format!("{}-Q", today.year());
        assert!(tag.starts_with(&expected_prefix));
        assert!(tag.ends_with(&expected_quarter.to_string()));
    }

    #[test]
    fn test_tag_name_format() {
        let tag = "2025-Q2";
        let full_ref = ArchiveTagger::<(u32,)>::tag_name(tag);
        assert_eq!(full_ref, "archive/2025-Q2");
    }

    // 下面的异步测试仅示例调用，不涉及 repo 操作
    #[tokio::test]
    #[ignore = "需要一个git仓库"]
    async fn test_compute_initial_delay_and_run() {
        let dummy_repo = GitBareRepository::new("gitnote.git"); // 这里需要有效路径或 mock

        let tagger = ArchiveTagger::new(dummy_repo, (4,), 24 * 3600);

        // 计算延迟应该不会 panic
        let delay = tagger.compute_initial_delay();
        println!("Computed initial delay: {:?}", delay);

        // 因为 run_scheduled_task 是无限循环，不能直接调用
        // 这里示范如何调用 maybe_tag_quarter_archive 同步方法
        // let _ = tagger.maybe_tag_quarter_archive();
    }

    #[tokio::test]
    #[ignore = "需要一个git仓库"]
    async fn test_push_tag() {
        let dummy_repo = GitBareRepository::new("gitnote.git"); // 这里需要有效路径或 mock

        let tagger = ArchiveTagger::new(dummy_repo, (4,), 24 * 3600);

        // 计算延迟应该不会 panic
        let delay = tagger.push_tag("archive/test");
        println!("Computed initial delay: {:?}", delay);
    }

    #[tokio::test]
    #[ignore = "需要一个git仓库"]
    async fn test_tag_quarter_archive() {
        let dummy_repo = GitBareRepository::new("gitnote.git"); // 这里需要有效路径或 mock

        let tagger = ArchiveTagger::new(dummy_repo, (4,), 24 * 3600);

        // 计算延迟应该不会 panic
        let delay = tagger.tag_quarter_archive();
        println!("Computed initial delay: {:?}", delay);
    }
}
