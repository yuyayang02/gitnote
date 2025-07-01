CREATE TABLE articles (
    slug TEXT PRIMARY KEY,            -- 通常为文件名去掉扩展名
    group_path TEXT NOT NULL,              -- 如 "posts/blog"
    title TEXT NOT NULL,                       -- front matter 中的标题
    summary TEXT NOT NULL,                     -- 摘要，可为 front matter 或正文提取
    tags TEXT[] NOT NULL,                        -- 可用逗号分隔，或用 JSON 存储
    content TEXT NOT NULL,            -- Markdown 正文内容
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL,    -- Git commit 时间或推送时间

    UNIQUE (group_path, slug)               -- 每组中文件唯一
);


CREATE TABLE groups (
    path TEXT PRIMARY KEY,
    public BOOLEAN NOT NULL DEFAULT false,         -- front matter 或组配置
    category JSONB DEFAULT NULL,
    author JSONB DEFAULT NULL
);