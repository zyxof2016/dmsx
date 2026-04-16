-- 公测 / 多租户冒烟：第二租户（与 docs/API.md 示例 UUID 对齐，便于文档与脚本一致）
INSERT INTO tenants (id, name) VALUES
    ('22222222-2222-2222-2222-222222222222', '公测租户 B')
ON CONFLICT (id) DO NOTHING;
