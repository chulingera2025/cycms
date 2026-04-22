import { useMemo } from 'react';
import { Alert, Button, Card, Col, Empty, Row, Space, Statistic, Tag, Typography } from 'antd';
import { Database, Feather, FileText, Image as ImageIcon, Puzzle, Settings, Sparkles, Users } from 'lucide-react';
import type { ReactNode } from 'react';
import { Link } from 'react-router-dom';
import { useContentList } from '@/features/content/hooks';
import { useStats } from '@/features/dashboard/hooks';
import { useAuth } from '@/stores/auth';
import type { ContentEntry } from '@/types';

interface CardDef {
  title: string;
  value: number;
  icon: ReactNode;
  to: string;
  color: string;
}

function entryLabel(entry: ContentEntry): string {
  const title = entry.fields.title;
  if (typeof title === 'string' && title.trim() !== '') {
    return title;
  }
  return entry.slug ?? entry.id.slice(0, 8);
}

function EntryListCard({
  title,
  description,
  entries,
  loading,
  empty,
  linkTo,
}: {
  title: string;
  description: string;
  entries: ContentEntry[];
  loading: boolean;
  empty: string;
  linkTo: string;
}) {
  return (
    <Card title={title} extra={<Link to={linkTo}>查看全部</Link>}>
      <Typography.Paragraph type="secondary" style={{ marginTop: 0 }}>
        {description}
      </Typography.Paragraph>

      {loading ? (
        <Typography.Text type="secondary">加载中...</Typography.Text>
      ) : entries.length === 0 ? (
        <Empty description={empty} image={Empty.PRESENTED_IMAGE_SIMPLE} />
      ) : (
        <div className="space-y-3">
          {entries.map((entry) => (
            <div key={entry.id} className="rounded-2xl border border-border bg-surface-alt px-4 py-3">
              <div className="flex items-start justify-between gap-3">
                <div>
                  <div className="font-medium text-text">{entryLabel(entry)}</div>
                  <div className="mt-1 text-xs text-text-muted">
                    {entry.slug ?? '无 slug'} · {new Date(entry.updated_at).toLocaleString('zh-CN')}
                  </div>
                </div>
                <Tag color={entry.status === 'published' ? 'green' : entry.status === 'draft' ? 'gold' : 'default'}>
                  {entry.status === 'published' ? '已发布' : entry.status === 'draft' ? '草稿' : '归档'}
                </Tag>
              </div>
            </div>
          ))}
        </div>
      )}
    </Card>
  );
}

export default function DashboardPage() {
  const { user } = useAuth();
  const stats = useStats();
  const draftParams = useMemo(
    () => ({ page: '1', pageSize: '5', status: 'draft', sort: 'updated_at:desc' }),
    [],
  );
  const publishedParams = useMemo(
    () => ({ page: '1', pageSize: '5', status: 'published', sort: 'published_at:desc' }),
    [],
  );
  const pageParams = useMemo(
    () => ({ page: '1', pageSize: '5', sort: 'updated_at:desc' }),
    [],
  );
  const settingsParams = useMemo(() => ({ page: '1', pageSize: '1' }), []);

  const draftPosts = useContentList('post', draftParams);
  const publishedPosts = useContentList('post', publishedParams);
  const pages = useContentList('page', pageParams);
  const siteSettings = useContentList('site_settings', settingsParams);
  const hasSiteSettings = (siteSettings.data?.meta.total ?? 0) > 0;

  const cards: CardDef[] = [
    {
      title: '内容模型',
      value: stats.contentTypes,
      icon: <Database size={18} />,
      to: '/admin/content-types',
      color: '#2563eb',
    },
    {
      title: '用户数',
      value: stats.users,
      icon: <Users size={18} />,
      to: '/admin/users',
      color: '#059669',
    },
    {
      title: '媒体资源',
      value: stats.media,
      icon: <ImageIcon size={18} />,
      to: '/admin/media',
      color: '#d97706',
    },
    {
      title: '启用插件',
      value: stats.plugins,
      icon: <Puzzle size={18} />,
      to: '/admin/plugins',
      color: '#7c3aed',
    },
  ];

  const quickLinks = [
    {
      title: '写文章',
      description: '进入 post 内容类型，开始新建或继续编辑文章。',
      to: '/admin/content?type=post',
      icon: <Feather size={16} />,
    },
    {
      title: '管理页面',
      description: '维护关于页、落地页和常驻页面内容。',
      to: '/admin/content?type=page',
      icon: <FileText size={16} />,
    },
    {
      title: '站点设置',
      description: '更新 site_settings，控制前台品牌文案和首页视觉。',
      to: '/admin/content?type=site_settings',
      icon: <Settings size={16} />,
    },
    {
      title: '媒体库',
      description: '上传封面、logo 和文章配图。',
      to: '/admin/media',
      icon: <ImageIcon size={16} />,
    },
  ];

  return (
    <div className="p-6">
      <section className="mb-6 rounded-[28px] border border-border bg-gradient-to-br from-brand/10 via-surface to-accent/10 px-6 py-6">
        <div className="flex flex-wrap items-end justify-between gap-4">
          <div>
            <div className="inline-flex items-center gap-2 rounded-full border border-brand/20 bg-brand/10 px-3 py-1 text-xs font-medium uppercase tracking-[0.18em] text-brand">
              <Sparkles size={14} />
              编辑工作台
            </div>
            <Typography.Title level={2} style={{ margin: '16px 0 6px' }}>
              欢迎回来{user?.username ? `，${user.username}` : ''}
            </Typography.Title>
            <Typography.Paragraph type="secondary" style={{ marginBottom: 0 }}>
              这里优先展示文章草稿、最近发布和站点准备情况，而不是只显示系统统计卡片。
            </Typography.Paragraph>
          </div>
          <Space wrap>
            <Link to="/admin/content?type=post">
              <Button type="primary" icon={<Feather size={14} />}>
                写文章
              </Button>
            </Link>
            <Link to="/admin/content?type=site_settings">
              <Button icon={<Settings size={14} />}>
                站点设置
              </Button>
            </Link>
          </Space>
        </div>
      </section>

      {!hasSiteSettings && (
        <Alert
          className="mb-6"
          type="info"
          showIcon
          message="站点设置尚未初始化"
          description="建议先创建 site_settings 条目，补齐站点名称、hero 文案、页脚文案和精选文章。"
        />
      )}

      <Row gutter={[16, 16]} className="mb-6">
        {cards.map((c) => (
          <Col key={c.title} xs={24} sm={12} lg={6}>
            <Link to={c.to} className="block no-underline">
              <Card hoverable>
                <div className="flex items-center justify-between gap-4">
                  <Statistic
                    title={c.title}
                    value={c.value}
                    loading={stats.loading}
                  />
                  <span
                    className="grid h-10 w-10 place-items-center rounded-full"
                    style={{ background: `${c.color}22`, color: c.color }}
                  >
                    {c.icon}
                  </span>
                </div>
              </Card>
            </Link>
          </Col>
        ))}
      </Row>

      <section className="mb-6">
        <Typography.Title level={4} style={{ marginTop: 0 }}>
          快速入口
        </Typography.Title>
        <Row gutter={[16, 16]}>
          {quickLinks.map((item) => (
            <Col key={item.title} xs={24} md={12} xl={6}>
              <Link to={item.to} className="block h-full no-underline">
                <Card hoverable className="h-full">
                  <div className="mb-3 inline-flex h-9 w-9 items-center justify-center rounded-full bg-brand/10 text-brand">
                    {item.icon}
                  </div>
                  <Typography.Title level={5} style={{ marginTop: 0, marginBottom: 8 }}>
                    {item.title}
                  </Typography.Title>
                  <Typography.Paragraph type="secondary" style={{ marginBottom: 0 }}>
                    {item.description}
                  </Typography.Paragraph>
                </Card>
              </Link>
            </Col>
          ))}
        </Row>
      </section>

      <Row gutter={[16, 16]}>
        <Col xs={24} xl={12}>
          <EntryListCard
            title="待完成草稿"
            description="按最近编辑时间排序，优先把未发布的文章推到首页。"
            entries={draftPosts.data?.data ?? []}
            loading={draftPosts.isLoading}
            empty="目前没有草稿文章。"
            linkTo="/admin/content?type=post"
          />
        </Col>
        <Col xs={24} xl={12}>
          <EntryListCard
            title="最近发布"
            description="按发布时间倒序，快速回看已经上线的文章。"
            entries={publishedPosts.data?.data ?? []}
            loading={publishedPosts.isLoading}
            empty="目前还没有已发布文章。"
            linkTo="/admin/content?type=post"
          />
        </Col>
        <Col xs={24} xl={12}>
          <Card title="页面与站点状态">
            <div className="space-y-3">
              <div className="rounded-2xl bg-surface-alt px-4 py-3">
                <div className="text-sm text-text-muted">页面数量</div>
                <div className="mt-1 text-2xl font-semibold text-text">{pages.data?.meta.total ?? 0}</div>
              </div>
              <div className="rounded-2xl bg-surface-alt px-4 py-3">
                <div className="text-sm text-text-muted">站点设置</div>
                <div className="mt-1 text-base font-medium text-text">
                  {hasSiteSettings ? '已配置' : '未配置'}
                </div>
              </div>
              <div className="rounded-2xl border border-dashed border-border px-4 py-3 text-sm text-text-secondary">
                默认博客预设已经落在 category、tag、page、post、site_settings 五类内容模型上。后台首页现在直接围绕这些模型组织工作流。
              </div>
            </div>
          </Card>
        </Col>
        <Col xs={24} xl={12}>
          <Card title="发布准备度">
            <div className="space-y-3">
              <div className="flex items-center justify-between rounded-2xl border border-border px-4 py-3">
                <span className="text-sm text-text">站点设置已创建</span>
                <Tag color={hasSiteSettings ? 'green' : 'gold'}>{hasSiteSettings ? '完成' : '待处理'}</Tag>
              </div>
              <div className="flex items-center justify-between rounded-2xl border border-border px-4 py-3">
                <span className="text-sm text-text">已发布文章</span>
                <Tag color={(publishedPosts.data?.meta.total ?? 0) > 0 ? 'green' : 'gold'}>
                  {publishedPosts.data?.meta.total ?? 0}
                </Tag>
              </div>
              <div className="flex items-center justify-between rounded-2xl border border-border px-4 py-3">
                <span className="text-sm text-text">媒体资源</span>
                <Tag color={stats.media > 0 ? 'green' : 'default'}>{stats.media}</Tag>
              </div>
              <div className="flex items-center justify-between rounded-2xl border border-border px-4 py-3">
                <span className="text-sm text-text">已启用插件</span>
                <Tag color={stats.plugins > 0 ? 'blue' : 'default'}>{stats.plugins}</Tag>
              </div>
            </div>
          </Card>
        </Col>
      </Row>
    </div>
  );
}
