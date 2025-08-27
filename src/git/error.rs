/// Git 操作统一错误类型。
///
/// 包含常见错误来源和场景：
///
/// - [`GitError::NotFound`]：请求的 blob 或文件不存在  
/// - [`GitError::NotExist`]：仓库不存在  
/// - [`GitError::InvalidConfig`]：仓库配置解析失败  
/// - [`GitError::Git2`]：底层 [`git2::Error`] 错误  
/// - [`GitError::IO`]：底层 IO 错误  
/// - [`GitError::CommandFailed`]：外部命令执行失败，包含错误信息
#[derive(Debug, thiserror::Error)]
pub enum GitError {
    /// 请求的 blob 或文件不存在
    #[error("blob not found")]
    NotFound,

    /// 仓库不存在
    #[error("repository not exist")]
    NotExist,

    /// 底层 git2 错误
    #[error("{0}")]
    Git2(#[from] git2::Error),

    /// 底层 IO 错误
    #[error(transparent)]
    IO(#[from] std::io::Error),

    /// git 命令执行失败
    #[error("command failed: {0}")]
    CommandFailed(String),
}
