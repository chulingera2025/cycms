import { Card, Col, Empty, Row, Statistic, Typography } from 'antd';
import { Database, Image as ImageIcon, Puzzle, Users } from 'lucide-react';
import type { ReactNode } from 'react';
import { Link } from 'react-router-dom';
import { useStats } from '@/features/dashboard/hooks';
import { useAuth } from '@/stores/auth';

interface CardDef {
  title: string;
  value: number;
  icon: ReactNode;
  to: string;
  color: string;
}

export default function DashboardPage() {
  const { user } = useAuth();
  const stats = useStats();

  const cards: CardDef[] = [
    {
      title: '内容类型',
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

  return (
    <div className="p-6">
      <div className="mb-6">
        <Typography.Title level={2} style={{ margin: 0 }}>
          仪表盘
        </Typography.Title>
        <Typography.Text type="secondary">
          欢迎回来{user?.username ? `，${user.username}` : ''}
        </Typography.Text>
      </div>

      <Row gutter={[16, 16]}>
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

      <Card className="mt-4" title="近期活动">
        <Empty description="暂无活动记录" />
      </Card>
    </div>
  );
}
