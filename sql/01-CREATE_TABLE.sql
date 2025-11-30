CREATE SCHEMA IF NOT EXISTS gitnote;

CREATE TABLE IF NOT EXISTS gitnote.articles (
    slug VARCHAR(255) PRIMARY KEY,                  -- 通常为文件名去掉扩展名
    title TEXT NOT NULL,                            -- front matter 中的标题
    summary TEXT NOT NULL,                          -- 摘要,可为 front matter 或正文提取
    tags TEXT[] NOT NULL,                           -- 可用逗号分隔,或用JSON存储
    content TEXT NOT NULL,                          -- Markdown 正文内容
    group_id VARCHAR(255) NOT NULL,                 -- 如 "posts/blog"
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,   -- 创建时间
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL,   -- 更新时间

    UNIQUE (group_id, slug)                         -- 每组中文件唯一
);


CREATE TABLE IF NOT EXISTS gitnote.groups (
    id VARCHAR(255) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    public BOOLEAN NOT NULL DEFAULT false,          -- front matter 或组配置
    kind JSONB DEFAULT '{}'::JSONB
);
